use std::{collections::VecDeque, sync::OnceLock};

use crate::repr::{
    bitboard::pop_lsb,
    board::{Board, FILES},
    move_gen::MoveGen,
    types::{BLACK, B_PAWN_U, WHITE, W_KING_U, W_PAWN_U},
};

use super::sliding_attacks;

// White owns the pawn and its file is reflected into a–d.
const STATES: usize = 24 * 64 * 64 * 2;
static DISTANCES: OnceLock<Box<[u8]>> = OnceLock::new();

pub(super) fn prepare(move_gen: &MoveGen) {
    DISTANCES.get_or_init(|| solve(move_gen));
}

pub(super) fn evaluate(board: &Board, move_gen: &MoveGen) -> Option<i16> {
    let pawns = board.pieces[W_PAWN_U] | board.pieces[B_PAWN_U];
    if pawns.count_ones() != 1 {
        return None;
    }

    let owner = if board.pieces[W_PAWN_U] != 0 {
        WHITE
    } else {
        BLACK
    };
    let mut flip = if owner == WHITE { 0 } else { 56 };
    let pawn = pawns.trailing_zeros() as usize;
    if pawn % 8 > 3 {
        flip ^= 7;
    }
    let state = State {
        pawn: pawn ^ flip,
        attacker: board.get_king_sqr_idx(owner) as usize ^ flip,
        defender: board.get_king_sqr_idx(owner ^ 1) as usize ^ flip,
        attacker_to_move: board.turn == owner,
    };
    if !(1..=6).contains(&(state.pawn / 8)) || !state.valid(move_gen) {
        return None;
    }

    let distances = DISTANCES.get_or_init(|| solve(move_gen));
    let distance = distances[state.index()];
    if distance == 0 {
        return Some(0);
    }

    // Keep a known win ahead of uncertain pawn endings and encourage progress to promotion.
    let score = 500 - 4 * i16::from(distance);
    Some(if state.attacker_to_move {
        score
    } else {
        -score
    })
}

#[derive(Clone, Copy)]
struct State {
    pawn: usize,
    attacker: usize,
    defender: usize,
    attacker_to_move: bool,
}

impl State {
    fn index(self) -> usize {
        let pawn = (self.pawn / 8 - 1) * 4 + self.pawn % 8;
        ((pawn * 64 + self.attacker) * 64 + self.defender) * 2 + usize::from(self.attacker_to_move)
    }

    fn from_index(index: usize) -> Self {
        let pawn = index >> 13;
        Self {
            pawn: (pawn / 4 + 1) * 8 + pawn % 4,
            attacker: index >> 7 & 63,
            defender: index >> 1 & 63,
            attacker_to_move: index & 1 != 0,
        }
    }

    fn valid(self, move_gen: &MoveGen) -> bool {
        self.attacker != self.defender
            && self.pawn != self.attacker
            && self.pawn != self.defender
            && king_moves(move_gen, self.attacker) & (1 << self.defender) == 0
            && (!self.attacker_to_move || !self.defender_in_check())
    }

    fn pawn_attacks(self) -> u64 {
        let pawn = 1u64 << self.pawn;
        ((pawn & !FILES[0]) << 7) | (pawn << 9)
    }

    fn defender_in_check(self) -> bool {
        self.pawn_attacks() & (1 << self.defender) != 0
    }

    fn defender_moves(self, move_gen: &MoveGen) -> u64 {
        king_moves(move_gen, self.defender)
            & !king_moves(move_gen, self.attacker)
            & !self.pawn_attacks()
    }

    fn winning_promotion(self, move_gen: &MoveGen) -> bool {
        if self.pawn / 8 != 6 {
            return false;
        }
        let promotion = self.pawn + 8;
        if promotion == self.attacker
            || promotion == self.defender
            || self.defender_moves(move_gen) & (1 << promotion) != 0
        {
            return false;
        }

        let rook = sliding_attacks(move_gen, 1 << self.attacker, promotion, true);
        let queen = rook | sliding_attacks(move_gen, 1 << self.attacker, promotion, false);
        let replies = king_moves(move_gen, self.defender) & !king_moves(move_gen, self.attacker);
        [queen, rook].into_iter().any(|attacks| {
            // Promotion wins if it delivers checkmate or leaves a legal king move.
            attacks & (1 << self.defender) != 0 || replies & !attacks != 0
        })
    }
}

#[inline]
fn king_moves(move_gen: &MoveGen, square: usize) -> u64 {
    move_gen.attack_bbs[W_KING_U][square]
}

fn solve(move_gen: &MoveGen) -> Box<[u8]> {
    let mut distances = vec![0; STATES].into_boxed_slice();
    let mut remaining = vec![0; STATES];
    let mut queue = VecDeque::new();
    for index in (0..STATES).step_by(2) {
        let defending = State::from_index(index);
        if !defending.valid(move_gen) {
            continue;
        }
        remaining[index] = defending.defender_moves(move_gen).count_ones() as u8;
        if defending.defender_in_check() {
            if remaining[index] == 0 {
                distances[index] = 1;
                queue.push_back(index);
            }
        } else if defending.winning_promotion(move_gen) {
            distances[index + 1] = 1;
            queue.push_back(index + 1);
        }
    }

    // A pawn-side turn wins through any winning child. A defender turn wins for
    // the pawn side only after every reply (including a possible capture) loses.
    while let Some(child_index) = queue.pop_front() {
        let child = State::from_index(child_index);
        let new_distance = distances[child_index] + 1;
        let mut visit = |parent: State| {
            if !parent.valid(move_gen) {
                return;
            }
            let index = parent.index();
            if distances[index] != 0 {
                return;
            }
            if !parent.attacker_to_move {
                remaining[index] -= 1;
                if remaining[index] != 0 {
                    return;
                }
            }
            distances[index] = new_distance;
            queue.push_back(index);
        };

        let mut parent = State {
            attacker_to_move: !child.attacker_to_move,
            ..child
        };
        if child.attacker_to_move {
            let mut origins = king_moves(move_gen, child.defender);
            while origins != 0 {
                parent.defender = pop_lsb(&mut origins) as usize;
                visit(parent);
            }
        } else {
            let mut origins = king_moves(move_gen, child.attacker);
            while origins != 0 {
                parent.attacker = pop_lsb(&mut origins) as usize;
                visit(parent);
            }
            parent.attacker = child.attacker;
            if child.pawn / 8 > 1 {
                parent.pawn = child.pawn - 8;
                visit(parent);
                if child.pawn / 8 == 3
                    && parent.pawn != child.attacker
                    && parent.pawn != child.defender
                {
                    parent.pawn -= 8;
                    visit(parent);
                }
            }
        }
    }
    distances
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitbase_classifies_all_normalized_positions() {
        let move_gen = MoveGen::init();
        let distances = DISTANCES.get_or_init(|| solve(&move_gen));
        let mut counts = [0; 3];
        for (index, &distance) in distances.iter().enumerate() {
            let valid = State::from_index(index).valid(&move_gen);
            let outcome = if !valid {
                assert_eq!(distance, 0);
                0
            } else if distance == 0 {
                1
            } else {
                2
            };
            counts[outcome] += 1;
        }

        // Invalid, drawn and won positions, reflecting the pawn into files a–d.
        assert_eq!(counts, [30_932, 54_394, 111_282]);
    }
}

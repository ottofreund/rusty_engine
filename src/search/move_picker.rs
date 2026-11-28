use crate::{
    repr::{_move, board::Board, move_gen::MoveGen, types::MAX_PSEUDO_MOVES_IN_POS},
    search::{eval::PIECE_MATERIAL_VALUE, search_data::SearchData},
};

const GOOD_TACTICAL: i32 = 32_000;
const FIRST_KILLER: i32 = 18_000;
const SECOND_KILLER: i32 = 17_000;
const BAD_TACTICAL: i32 = -26_000;

/// Picks the next move in place. Scores are initialized only if the preferred
/// moves fail to cut off, and SEE runs only when a tactical move is selected.
pub(super) struct MovePicker {
    priority: [u32; 2],
    ply: usize,
    continuation_context: [Option<usize>; 2],
    scores: [i32; MAX_PSEUDO_MOVES_IN_POS],
    scored: bool,
}

impl MovePicker {
    pub(super) fn new(ply: usize, primary: u32, secondary: u32, played_moves: &[u32]) -> Self {
        Self {
            priority: [primary, secondary],
            ply,
            continuation_context: SearchData::continuation_context(played_moves),
            scores: [0; MAX_PSEUDO_MOVES_IN_POS],
            scored: false,
        }
    }

    pub(super) fn next(
        &mut self,
        moves: &mut [u32],
        index: usize,
        move_gen: &MoveGen,
        data: &SearchData,
        board: &Board,
    ) -> u32 {
        for priority in &mut self.priority {
            let wanted = std::mem::replace(priority, _move::NULL_MOVE);
            if wanted == _move::NULL_MOVE {
                continue;
            }
            if let Some(offset) = moves[index..]
                .iter()
                .position(|&mov| _move::move_eq(mov, wanted))
            {
                moves.swap(index, index + offset);
                moves[index] = Self::classify_tactical(moves[index], data, board, move_gen);
                return moves[index];
            }
        }

        if !self.scored {
            let killers = data
                .killers
                .get(self.ply)
                .copied()
                .unwrap_or([_move::NULL_MOVE; 2]);
            for (i, &mov) in moves.iter().enumerate().skip(index) {
                self.scores[i] = if _move::is_eating(mov) || _move::is_promotion(mov) {
                    let stage = if _move::is_negative_see(mov) {
                        BAD_TACTICAL
                    } else {
                        GOOD_TACTICAL
                    };
                    stage + Self::tactical_score(mov, data)
                } else if mov == killers[0] {
                    FIRST_KILLER
                } else if mov == killers[1] {
                    SECOND_KILLER
                } else {
                    self.quiet_history(data, board.turn, mov)
                };
            }
            self.scored = true;
        }

        loop {
            let mut best = index;
            for i in index + 1..moves.len() {
                if self.scores[i] > self.scores[best] {
                    best = i;
                }
            }
            let mov = moves[best];
            let classified = Self::classify_tactical(mov, data, board, move_gen);
            moves[best] = classified;
            if classified != mov && _move::is_negative_see(classified) {
                self.scores[best] += BAD_TACTICAL - GOOD_TACTICAL;
                continue;
            }
            moves.swap(index, best);
            self.swap_scores(index, best);
            return classified;
        }
    }

    pub(super) fn swap_scores(&mut self, first: usize, second: usize) {
        if self.scored {
            self.scores.swap(first, second);
        }
    }

    pub(super) fn quiet_history(&self, data: &SearchData, side: u32, mov: u32) -> i32 {
        data.quiet_history(side, mov, self.continuation_context)
    }

    fn tactical_score(mov: u32, data: &SearchData) -> i32 {
        let value = |piece: u32| i32::from(PIECE_MATERIAL_VALUE[piece as usize]);
        let victim = _move::eaten_piece(mov).map_or(0, value);
        let promotion = _move::lift_promotion_piece(mov)
            .map_or(0, |piece| value(piece) - value(_move::get_moved_piece(mov)));
        let attacker = value(_move::get_moved_piece(mov)).min(1_000);
        8 * (victim + promotion) - attacker + data.capture_history(mov) / 2
    }

    fn classify_tactical(mov: u32, data: &SearchData, board: &Board, move_gen: &MoveGen) -> u32 {
        if _move::is_positive_see(mov) || _move::is_negative_see(mov) {
            return mov;
        }
        if _move::is_eating(mov) || _move::is_promotion(mov) {
            if data.see_helper.see_positive(mov, board, move_gen) {
                _move::with_positive_see(mov)
            } else {
                _move::with_negative_see(mov)
            }
        } else {
            mov
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{repr::position::Position, utils::zobrist::Zobrist};

    #[test]
    fn good_captures_and_killers_precede_losing_captures() {
        let move_gen = MoveGen::init();
        let pos = Position::from(
            "4k3/8/5p2/4p3/3Q4/8/p7/R3K3 w - - 0 1",
            &move_gen,
            &Zobrist::default(),
        )
        .unwrap();
        let mut data = SearchData::new(&pos);
        let mut moves = pos.legal_moves().to_vec();
        let find = |name| {
            *pos.legal_moves()
                .iter()
                .find(|&&mov| _move::to_string(mov, true) == name)
                .unwrap()
        };
        let good = find("a1a2");
        let bad = find("d4e5");
        let killer = find("e1f1");
        data.killers[0][0] = killer;

        let mut picker = MovePicker::new(0, _move::NULL_MOVE, _move::NULL_MOVE, &[]);
        let first = picker.next(&mut moves, 0, &move_gen, &data, &pos.board);
        assert!(_move::move_eq(first, good));
        assert!(_move::is_positive_see(first));
        assert_eq!(
            picker.next(&mut moves, 1, &move_gen, &data, &pos.board),
            killer
        );

        let mut picker = MovePicker::new(0, bad, good, &[]);
        let first = picker.next(&mut moves, 0, &move_gen, &data, &pos.board);
        assert!(_move::move_eq(first, bad));
        assert!(_move::is_negative_see(first));
        assert!(_move::move_eq(
            picker.next(&mut moves, 1, &move_gen, &data, &pos.board),
            good
        ));
    }
}

use crate::{
    repr::{
        _move, bitboard,
        board::{Board, FILES},
        move_gen::MoveGen,
        types::*,
    },
    search::eval::PIECE_MATERIAL_VALUE,
};

#[derive(Default)]
pub struct SeeWorker;

impl SeeWorker {
    #[inline]
    pub fn see_positive(&self, mov: u32, board: &Board, move_gen: &MoveGen) -> bool {
        self.see_ge(mov, 0, board, move_gen)
    }

    /// Tests whether the material gain of a legal move and subsequent exchanges
    /// on its destination reaches the threshold. Recaptures use the least
    /// valuable available attacker and either side can decline to capture.
    pub fn see_ge(&self, mov: u32, threshold: i32, board: &Board, move_gen: &MoveGen) -> bool {
        let mover = board.turn;
        let source = _move::get_init(mov);
        let target = _move::get_target(mov) as usize;
        let moved = _move::get_moved_piece(mov);
        let placed = _move::lift_promotion_piece(mov).unwrap_or(moved);
        let value = |piece: u32| i32::from(PIECE_MATERIAL_VALUE[piece as usize]);
        let mut gain = _move::eaten_piece(mov).map_or(0, value) + value(placed) - value(moved);
        if gain < threshold {
            return false;
        }
        if moved % NOF_PIECE_TYPES == W_KING
            || gain - value(placed) >= threshold && !promotes_on(mover ^ 1, target)
        {
            return true;
        }

        // The target is occupied throughout the exchange, but its successive
        // occupants are tracked by value instead of in the original piece sets.
        let mut occupied = board.total_occupation() & !(1u64 << source) & !(1u64 << target);
        if _move::is_en_passant(mov) {
            let victim = if mover == WHITE {
                target - 8
            } else {
                target + 8
            };
            occupied &= !(1u64 << victim);
        }
        let occupation = [board.white_occupation, board.black_occupation];
        let mut side = mover ^ 1;
        let mut occupant_value = value(placed);
        loop {
            let attackers = attacks_to(target, occupied, board, move_gen);
            let Some((origin, kind)) = least_valuable_attacker(
                attackers & occupation[side as usize],
                target,
                side,
                occupied,
                board,
                move_gen,
            ) else {
                return gain >= threshold;
            };
            occupied &= !(1u64 << origin);

            let promotes = kind == W_PAWN_U && promotes_on(side, target);
            let next_value =
                i32::from(PIECE_MATERIAL_VALUE[if promotes { W_QUEEN_U } else { kind }]);
            let promotion_gain = if promotes {
                next_value - value(W_PAWN)
            } else {
                0
            };
            let capture_gain = occupant_value + promotion_gain;
            gain += if side == mover {
                capture_gain
            } else {
                -capture_gain
            };
            if kind == W_KING_U {
                return gain >= threshold;
            }

            // The next side can stand pat as soon as the current total meets
            // its objective. Earlier captures can only move the opposite way.
            let reaches_threshold = gain >= threshold;
            if reaches_threshold != (side == mover) {
                return reaches_threshold;
            }
            occupant_value = next_value;
            side ^= 1;
        }
    }
}

#[inline]
fn promotes_on(side: u32, target: usize) -> bool {
    target >> 3 == if side == WHITE { 7 } else { 0 }
}

#[inline]
fn attacks_to(target: usize, occupied: u64, board: &Board, move_gen: &MoveGen) -> u64 {
    let bit = 1u64 << target;
    let white_pawns = ((bit & !FILES[0]) >> 9) | ((bit & !FILES[7]) >> 7);
    let black_pawns = ((bit & !FILES[0]) << 7) | ((bit & !FILES[7]) << 9);
    let queens = board.pieces[W_QUEEN_U] | board.pieces[B_QUEEN_U];
    let diagonals = board.pieces[W_BISHOP_U] | board.pieces[B_BISHOP_U] | queens;
    let straights = board.pieces[W_ROOK_U] | board.pieces[B_ROOK_U] | queens;
    let knights = board.pieces[W_KNIGHT_U] | board.pieces[B_KNIGHT_U];
    let kings = board.pieces[W_KING_U] | board.pieces[B_KING_U];

    (white_pawns & board.pieces[W_PAWN_U]
        | black_pawns & board.pieces[B_PAWN_U]
        | move_gen.attack_bbs[W_KNIGHT_U][target] & knights
        | move_gen.attack_bbs[W_KING_U][target] & kings)
        & occupied
        | slider_attackers(target, occupied, diagonals, straights, move_gen)
}

#[inline]
fn slider_attackers(
    target: usize,
    occupied: u64,
    diagonals: u64,
    straights: u64,
    move_gen: &MoveGen,
) -> u64 {
    let diagonals = diagonals & occupied & move_gen.attack_bbs[W_BISHOP_U][target];
    let straights = straights & occupied & move_gen.attack_bbs[W_ROOK_U][target];
    let mut attackers = 0;
    if diagonals != 0 {
        let blockers = move_gen.get_relevant_blockers(target, occupied, false);
        attackers |= diagonals & move_gen.get_sliding_for(target, blockers, false);
    }
    if straights != 0 {
        let blockers = move_gen.get_relevant_blockers(target, occupied, true);
        attackers |= straights & move_gen.get_sliding_for(target, blockers, true);
    }
    attackers
}

fn least_valuable_attacker(
    attackers: u64,
    target: usize,
    side: u32,
    occupied: u64,
    board: &Board,
    move_gen: &MoveGen,
) -> Option<(u32, usize)> {
    if attackers == 0 {
        return None;
    }

    let own = side as usize * NOF_PIECE_TYPES_U;
    let enemy = (side ^ 1) as usize * NOF_PIECE_TYPES_U;
    let queens = board.pieces[enemy + W_QUEEN_U];
    let diagonals = board.pieces[enemy + W_BISHOP_U] | queens;
    let straights = board.pieces[enemy + W_ROOK_U] | queens;
    let king = board.get_king_sqr_idx(side) as usize;
    let opponent = if side == WHITE {
        board.black_occupation
    } else {
        board.white_occupation
    };
    for kind in W_PAWN_U..=W_KING_U {
        let mut candidates = attackers & board.pieces[own + kind];
        while candidates != 0 {
            let origin = bitboard::pop_lsb(&mut candidates);
            let remaining = occupied & !(1u64 << origin);
            let safe = if kind == W_KING_U {
                attacks_to(target, remaining, board, move_gen) & opponent == 0
            } else {
                // The capturing piece on the destination can block a pin, but
                // the captured opponent piece is absent from the slider sets.
                slider_attackers(
                    king,
                    remaining | (1u64 << target),
                    diagonals & remaining,
                    straights & remaining,
                    move_gen,
                ) == 0
            };
            if safe {
                return Some((origin, kind));
            }
        }
    }
    None
}

#[cfg(test)]
#[path = "../../tests/unit/see_tests.rs"]
mod tests;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::repr::{
    _move, bitboard, board::Board, move_gen::MoveGen, position::Position,
    types::{BLACK, B_KING_U, B_PAWN_U, B_ROOK_U, NOF_PIECE_TYPES, WHITE, W_KING_U, W_PAWN_U, W_ROOK_U},
};

const PIECE_RANDS_LEN: usize = NOF_PIECE_TYPES as usize * 2 * 64;
const NOF_SQUARES: usize = 64;
const ROW_LEN: usize = 8;
const KEY50_SALT_START_CLOCK: u32 = 50;

pub struct Zobrist {
    piece_rands: [u64; PIECE_RANDS_LEN],
    en_passant_file_rands: [u64; 8],
    black_turn_rand: u64,
    ws_rand: u64,
    wl_rand: u64,
    bs_rand: u64,
    bl_rand: u64,
}

impl Zobrist {
    pub fn init_hash(&self, board: &Board, has_ep_take: bool) -> u64 {
        let mut h: u64 = 0;

        for p in 0..board.pieces.len() {
            let mut bb: u64 = board.pieces[p];
            while bb > 0 {
                let sqr: u32 = bitboard::pop_lsb(&mut bb);
                h ^= self.piece_rands[p * NOF_SQUARES + sqr as usize];
            }
        }

        if board.turn == BLACK {
            h ^= self.black_turn_rand;
        }
        if board.ws() {
            h ^= self.ws_rand;
        }
        if board.wl() {
            h ^= self.wl_rand;
        }
        if board.bs() {
            h ^= self.bs_rand;
        }
        if board.bl() {
            h ^= self.bl_rand;
        }
        if has_ep_take {
            h ^= self.en_passant_file_rands[board.ep_square.expect("has_ep_take is true but ep_square is None") as usize % 8];
        }

        return h;
    }

    /// TT key after a legal non-checking move with no capture, promotion or castle.
    pub(crate) fn quiet_move_tt_key(&self, pos: &Position, mov: u32, move_gen: &MoveGen) -> u64 {
        let board = &pos.board;
        let from = _move::get_init(mov) as usize;
        let to = _move::get_target(mov) as usize;
        let piece = _move::get_moved_piece(mov) as usize;
        let is_white = board.turn == WHITE;
        let allows_ep = _move::is_double_push(mov)
            && move_gen.has_ep_after_quiet_double_push(board, from, to);
        let old_ep = if pos.has_ep_take { board.ep_square } else { None };
        // A quiet move cannot land on a rook home square with an active right.
        let key = self.updated_hash_forward(
            pos.zhash,
            from,
            to,
            piece,
            is_white,
            false,
            None,
            false,
            None,
            false,
            false,
            allows_ep,
            false,
            is_white && (from == 7 || piece == W_KING_U) && board.ws(),
            is_white && (from == 0 || piece == W_KING_U) && board.wl(),
            !is_white && (from == 63 || piece == B_KING_U) && board.bs(),
            !is_white && (from == 56 || piece == B_KING_U) && board.bl(),
            old_ep,
        );
        let clock = if piece == W_PAWN_U || piece == B_PAWN_U {
            0
        } else {
            board.half_move_clock + 1
        };
        Self::zkey50_adjusted(key, clock)
    }

    ///when making move
    ///lost_ep <==> if position before making this move had ep sqr, then Some(ep_sqr) else None
    pub fn updated_hash_forward(
        &self,
        cur: u64,
        from: usize,
        to: usize,
        moved_piece: usize,
        is_white_turn: bool,
        is_promotion: bool,
        promotion_piece: Option<usize>,
        is_eating: bool,
        eaten_piece: Option<usize>,
        is_castle: bool,
        is_short_castle: bool,
        allows_ep_take: bool,
        is_en_passant: bool,
        lost_ws: bool,
        lost_wl: bool,
        lost_bs: bool,
        lost_bl: bool,
        lost_ep_take: Option<u32>,
    ) -> u64 {
        let mut new: u64 = cur;

        if is_eating && !is_en_passant {
            //clear eaten piece, en passant has own clearing logic
            let eaten_piece: usize = eaten_piece.expect("Was eating but no eating piece found");
            new ^= self.piece_rands[eaten_piece * NOF_SQUARES + to];
        }
        new ^= self.piece_rands[moved_piece * NOF_SQUARES + from];
        if !is_promotion {
            new ^= self.piece_rands[moved_piece * NOF_SQUARES + to];
        }

        if is_promotion {
            let promotion_piece: usize =
                promotion_piece.expect("Was promotion but no promotion piece found");
            new ^= self.piece_rands[promotion_piece * NOF_SQUARES + to];
        } else if is_castle {
            let rook_from: usize;
            let rook_to: usize;
            let rook_piece_idx: usize;
            if is_short_castle {
                if is_white_turn {
                    rook_from = 7;
                    rook_to = 5;
                    rook_piece_idx = W_ROOK_U;
                } else {
                    rook_from = 63;
                    rook_to = 61;
                    rook_piece_idx = B_ROOK_U;
                }
            } else {
                if is_white_turn {
                    rook_from = 0;
                    rook_to = 3;
                    rook_piece_idx = W_ROOK_U;
                } else {
                    rook_from = 56;
                    rook_to = 59;
                    rook_piece_idx = B_ROOK_U;
                }
            }
            new ^= self.piece_rands[rook_piece_idx * NOF_SQUARES + rook_from];
            new ^= self.piece_rands[rook_piece_idx * NOF_SQUARES + rook_to];
        }

        if lost_ws {
            new ^= self.ws_rand;
        }
        if lost_wl {
            new ^= self.wl_rand;
        }
        if lost_bs {
            new ^= self.bs_rand;
        }
        if lost_bl {
            new ^= self.bl_rand;
        }

        if is_en_passant {
            //clear ep_square piece
            let opponent_pawn_idx: usize;
            let offset: usize;
            if is_white_turn {
                opponent_pawn_idx = B_PAWN_U;
                offset = 4 * ROW_LEN
            } else {
                opponent_pawn_idx = W_PAWN_U;
                offset = 3 * ROW_LEN;
            }
            let eating_sqr: usize =
                lost_ep_take.expect("Was ep but lost_ep_take was None") as usize % 8 + offset; //file idx + row offset
            new ^= self.piece_rands[opponent_pawn_idx * NOF_SQUARES + eating_sqr];
        }

        if allows_ep_take {
            new ^= self.en_passant_file_rands[to % 8];
        }

        if let Some(ep_square) = lost_ep_take {
            new ^= self.en_passant_file_rands[ep_square as usize % 8];
        }

        new ^= self.black_turn_rand;
        return new;
    }

    /// when unmaking move
    /// gained_ep <==> if position after unmaking this move has ep sqr, then Some(ep_sqr) else None
    pub fn updated_hash_backward(
        &self,
        cur: u64,
        from: usize,
        to: usize,
        moved_piece: usize,
        is_white_turn: bool,
        is_promotion: bool,
        promotion_piece: Option<usize>,
        is_eating: bool,
        eaten_piece: Option<usize>,
        is_castle: bool,
        is_short_castle: bool,
        is_en_passant: bool,
        gained_ws: bool,
        gained_wl: bool,
        gained_bs: bool,
        gained_bl: bool,
        gained_ep_take: Option<u32>,
        lost_ep_take: Option<u32>
    ) -> u64 {
        let mut new: u64 = cur;

        if is_eating && !is_en_passant {
            let eaten_piece: usize = eaten_piece.expect("Was eating but no eating piece found");
            new ^= self.piece_rands[eaten_piece * NOF_SQUARES + to];
        }

        new ^= self.piece_rands[moved_piece * NOF_SQUARES + from];

        if !is_promotion {
            new ^= self.piece_rands[moved_piece * NOF_SQUARES + to];
        }

        if is_promotion {
            let promotion_piece: usize =
                promotion_piece.expect("Was promotion but no promotion piece found");
            new ^= self.piece_rands[promotion_piece * NOF_SQUARES + to];
        } else if is_castle {
            let rook_from: usize;
            let rook_to: usize;
            let rook_piece_idx: usize;

            if is_short_castle {
                if is_white_turn {
                    rook_from = 7;
                    rook_to = 5;
                    rook_piece_idx = W_ROOK_U;
                } else {
                    rook_from = 63;
                    rook_to = 61;
                    rook_piece_idx = B_ROOK_U;
                }
            } else {
                if is_white_turn {
                    rook_from = 0;
                    rook_to = 3;
                    rook_piece_idx = W_ROOK_U;
                } else {
                    rook_from = 56;
                    rook_to = 59;
                    rook_piece_idx = B_ROOK_U;
                }
            }

            new ^= self.piece_rands[rook_piece_idx * NOF_SQUARES + rook_from];
            new ^= self.piece_rands[rook_piece_idx * NOF_SQUARES + rook_to];
        }

        if gained_ws {
            new ^= self.ws_rand;
        }
        if gained_wl {
            new ^= self.wl_rand;
        }
        if gained_bs {
            new ^= self.bs_rand;
        }
        if gained_bl {
            new ^= self.bl_rand;
        }

        if is_en_passant {
            let opponent_pawn_idx: usize;
            let offset: usize;

            if is_white_turn {
                opponent_pawn_idx = B_PAWN_U;
                offset = 4 * ROW_LEN;
            } else {
                opponent_pawn_idx = W_PAWN_U;
                offset = 3 * ROW_LEN;
            }

            let eating_sqr: usize =
                gained_ep_take.expect("Unmaking ep but gained_ep was None") as usize % 8 + offset;

            new ^= self.piece_rands[opponent_pawn_idx * NOF_SQUARES + eating_sqr];
        }

        if let Some(ep_square) = gained_ep_take {
            new ^= self.en_passant_file_rands[ep_square as usize % 8];
        }

        if let Some(ep_square) = lost_ep_take {
            new ^= self.en_passant_file_rands[ep_square as usize % 8];
        }

        new ^= self.black_turn_rand;

        return new;
    }

    #[inline]
    /// Taken from Stockfish
    pub fn zkey50_adjusted(key: u64, half_move_clock: u32) -> u64 {
        if half_move_clock < KEY50_SALT_START_CLOCK {
            return key;
        } else {
            return key ^ Zobrist::make_key((half_move_clock as u64 - KEY50_SALT_START_CLOCK as u64) / 8);
        }
    }

    /// Taken from Stockfish
    #[inline]
    fn make_key(seed: u64) -> u64 {
        seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
    }

}

impl Default for Zobrist {
    fn default() -> Self {
        let mut rng: StdRng = StdRng::seed_from_u64(12345);
        let piece_rands: [u64; PIECE_RANDS_LEN] = std::array::from_fn(|_| rng.random::<u64>());
        let en_passant_file_rands: [u64; 8] = std::array::from_fn(|_| rng.random::<u64>());
        return Self {
            piece_rands,
            en_passant_file_rands,
            black_turn_rand: rng.random::<u64>(),
            ws_rand: rng.random::<u64>(),
            wl_rand: rng.random::<u64>(),
            bs_rand: rng.random::<u64>(),
            bl_rand: rng.random::<u64>(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_move_tt_key_matches_make_move() {
        let move_gen = MoveGen::init();
        let zobrist = Zobrist::default();
        for fen in [
            "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 49 1",
            "r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 57 1",
            "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1",
            "4k3/8/8/8/3p4/8/4P3/4K3 w - - 57 1",
            "k7/3p4/8/4P3/8/8/8/4K3 b - - 49 1",
            "k3r3/3p4/8/4P3/8/8/8/4K3 b - - 49 1",
            "8/5p2/8/r5PK/8/8/8/k7 b - - 0 1",
        ] {
            let pos = Position::from(fen, &move_gen, &zobrist).unwrap();
            let info = move_gen.check_info(&pos.board);
            for &mov in pos.legal_search_moves() {
                if _move::is_eating(mov)
                    || _move::is_promotion(mov)
                    || _move::is_castle(mov)
                    || move_gen.gives_check(&pos.board, &info, mov)
                {
                    continue;
                }
                let key = zobrist.quiet_move_tt_key(&pos, mov, &move_gen);
                let mut child = pos.clone();
                child.make_move(mov, true, false, true, &move_gen, &zobrist);
                assert_eq!(
                    key,
                    Zobrist::zkey50_adjusted(child.zhash, child.board.half_move_clock),
                    "{fen} after {}",
                    _move::to_string(mov, true)
                );
            }
        }
    }
}

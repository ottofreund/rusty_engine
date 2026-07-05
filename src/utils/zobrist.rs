use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::repr::bitboard;
use crate::repr::board::Board;
use crate::repr::types::{BLACK, NOF_PIECE_TYPES};

const PIECE_RANDS_LEN: usize = NOF_PIECE_TYPES as usize * 2 * 64;
const NOF_SQUARES: usize = 64;
const ROW_LEN: usize = 8;

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
    pub fn init_hash(&self, board: &Board) -> u64 {
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
        if let Some(ep_square) = board.ep_square {
            h ^= self.en_passant_file_rands[ep_square as usize % 8];
        }

        return h;
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
        is_double_push: bool,
        is_en_passant: bool,
        lost_ws: bool,
        lost_wl: bool,
        lost_bs: bool,
        lost_bl: bool,
        lost_ep: Option<u32>,
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
                    rook_piece_idx = 3;
                } else {
                    rook_from = 63;
                    rook_to = 61;
                    rook_piece_idx = 9;
                }
            } else {
                if is_white_turn {
                    rook_from = 0;
                    rook_to = 3;
                    rook_piece_idx = 3;
                } else {
                    rook_from = 56;
                    rook_to = 59;
                    rook_piece_idx = 9;
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
            //clear ep_square
            let opponent_pawn_idx: usize;
            let offset: usize;
            if is_white_turn {
                opponent_pawn_idx = 6;
                offset = 4 * ROW_LEN
            } else {
                opponent_pawn_idx = 0;
                offset = 3 * ROW_LEN;
            }
            let eating_sqr: usize =
                lost_ep.expect("Was ep but lost_ep was None") as usize % 8 + offset; //file idx + row offset
            new ^= self.piece_rands[opponent_pawn_idx * NOF_SQUARES + eating_sqr];
        }

        if is_double_push {
            new ^= self.en_passant_file_rands[to % 8];
        }

        if let Some(ep_square) = lost_ep {
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
        is_double_push: bool,
        is_en_passant: bool,
        gained_ws: bool,
        gained_wl: bool,
        gained_bs: bool,
        gained_bl: bool,
        gained_ep: Option<u32>,
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
                    rook_piece_idx = 3;
                } else {
                    rook_from = 63;
                    rook_to = 61;
                    rook_piece_idx = 9;
                }
            } else {
                if is_white_turn {
                    rook_from = 0;
                    rook_to = 3;
                    rook_piece_idx = 3;
                } else {
                    rook_from = 56;
                    rook_to = 59;
                    rook_piece_idx = 9;
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
                opponent_pawn_idx = 6;
                offset = 4 * ROW_LEN;
            } else {
                opponent_pawn_idx = 0;
                offset = 3 * ROW_LEN;
            }

            let eating_sqr: usize =
                gained_ep.expect("Was ep but gained_ep was None") as usize % 8 + offset;

            new ^= self.piece_rands[opponent_pawn_idx * NOF_SQUARES + eating_sqr];
        }

        if is_double_push {
            new ^= self.en_passant_file_rands[to % 8];
        }

        if let Some(ep_square) = gained_ep {
            new ^= self.en_passant_file_rands[ep_square as usize % 8];
        }

        new ^= self.black_turn_rand;

        return new;
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

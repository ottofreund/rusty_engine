pub const W_PAWN: u32 = 0;
pub const W_KNIGHT: u32 = 1;
pub const W_BISHOP: u32 = 2;
pub const W_ROOK: u32 = 3;
pub const W_QUEEN: u32 = 4;
pub const W_KING: u32 = 5;
pub const B_PAWN: u32 = 6;
pub const B_KNIGHT: u32 = 7;
pub const B_BISHOP: u32 = 8;
pub const B_ROOK: u32 = 9;
pub const B_QUEEN: u32 = 10;
pub const B_KING: u32 = 11;

pub const WHITE: u32 = 0;
pub const BLACK: u32 = 1;

pub const NOF_PIECE_TYPES: u32 = 6;

pub const MAX_PSEUDO_MOVES_IN_POS: usize = 512; //approximate power of two

pub fn opposite_turn(color: u32) -> u32 {
    return color ^ 1;
}

#[derive(Clone, Debug)]
pub struct BoardStateInfo {
    pub ep_sqr: Option<u32>,
    pub nof_checkers: u32,
    pub check_block_sqrs: u64,
    pub mover_pinned: u64,
    pub mover_pinned_restrictions: [u64; 64],
    pub meta_attacks: u64,
    pub opponent_attacked: u64,
    pub half_move_clock: u32,
}

/* Utils for u64 (magic) bitboards  */

use crate::repr::board::{A_FILE, H_FILE, RANK_1, RANK_8};
use crate::repr::move_gen::MoveGen;
use crate::repr::types::W_ROOK;

///returns idx of toggled lsb and toggles off that square for the passed mutable reference
pub fn pop_lsb(bb: &mut u64) -> u32 {
    let trailing: u32 = bb.trailing_zeros();
    *bb &= *bb - 1; 
    return trailing;
}
///returns new bb with toggled off lsb for passed bb
pub fn with_pop_lsb(bb: u64) -> u64 {
    return bb & (bb - 1); 
}
///Set bit at sqr to 1 mutably to passed bb
pub fn set_square(bb: &mut u64, sqr: u32) {
    *bb |= 1 << sqr;
}
///Return new bitboard with set bit at sqr
pub fn with_set_square(bb: u64, sqr: u32) -> u64 {
    return bb | (1 << sqr);
}
///Clear bit at sqr to 0
pub fn clear_square(bb: &mut u64, sqr: u32) {
    *bb &= !(1 << sqr);
}
///Does bb have sqr toggled?
pub fn contains_square(bb: u64, sqr: u32) -> bool {
    return bb & (1 << sqr) != 0;
}
///Remove
pub fn diff(bb: u64, bb_to_exclude: u64) -> u64 {
    return bb & !bb_to_exclude;
}
///Get all relevant blockers for blocker mask. This means excluding edge squares on edges that sqr is not on.
/// scan_cardinal for rook, scan_diag for bishop, both for queen
/// Very good constant time complexity due to memoization of no-edge bitboards
pub fn get_relevant_blockers(sqr: u32, blockers: u64, scan_cardinal: bool, scan_diag: bool, move_gen: &MoveGen) -> u64 {
    let mut res: u64 = 0;
    if scan_cardinal {
        res |= move_gen.rook_bbs_no_edges[sqr as usize];
    }
    if scan_diag {
        res |= move_gen.bishop_bbs_no_edges[sqr as usize];
    }
    return res & blockers;
}

pub fn bb_to_string(bb: u64) -> String {
    let mut res: String = String::new();
    let mut i: i32 = 56;
    while i >= 0 {
        let bit_val: u64 = (bb >> i) & 1;
        res += &bit_val.to_string();
        res += " ";
        if (i + 1) % 8 == 0 { //end of row, go to left of bottom row
            res += "\n";
            i -= 15;
        } else {
            i += 1;
        }
    }
    return res;
}
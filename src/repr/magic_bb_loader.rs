use std::collections::HashSet;
use rand::prelude::*;
use crate::repr::bitboard;
use crate::repr::move_gen::{self, naive_rook_sliding, naive_bishop_sliding};
///MagicBitboard object is initialized on startup and holds all relevant data and methods related to initializing magic bitboards and using them.
pub struct MagicBitboard {
    pub rook_slide_bbs: Vec<Vec<u64>>, //'legal' slides magically indexed by **relevant** blocker masks
    pub bishop_slide_bbs: Vec<Vec<u64>>, //vec because multi-megabyte array too large for stack
    rook_magics: [u64 ; 64], //rook magic multipliers for each square
    bishop_magics: [u64 ; 64] //for bishop
}

impl MagicBitboard {
    ///Initializes magic bitboards by finding a working magic number (multiplier and shift amount) for each square. Returns Self which provides API to easily fetch 'legal' slide bbs magically indexed
    pub fn init_magic(empty_board_attack_bbs: &[[u64 ; 64] ; 12], rook_empty_board_attack_bbs_no_edges: &[u64 ; 64], bishop_empty_board_attack_bbs_no_edges: &[u64 ; 64]) -> Self {
        let mut rook_magics: [u64 ; 64] = [0 ; 64];
        let mut bishop_magics: [u64 ; 64] = [0 ; 64];
        let mut rook_slide_bbs: Vec<Vec<u64>> = Vec::new(); //these are filled "on the fly" square by square so that each square reserves as little space as it actually needs (squares have varying need of nof bits == inner vec len)
        let mut bishop_slide_bbs: Vec<Vec<u64>> = Vec::new();

        let mut rng: ThreadRng = rand::rng();
        for sqr in 0..64 {  
            let rook_empty_board_attack_bb: u64 = empty_board_attack_bbs[3][sqr];
            let rook_empty_board_attack_bb_no_edges: u64 = rook_empty_board_attack_bbs_no_edges[sqr];
            let bishop_empty_board_attack_bb: u64 = empty_board_attack_bbs[2][sqr as usize];
            let bishop_empty_board_attack_bb_no_edges: u64 = bishop_empty_board_attack_bbs_no_edges[sqr];
            //we don't want edges here with the following, lookups are done with relevant blocker bitboards
            let all_rook_block_masks: Vec<u64> = move_gen::generate_all_blocker_masks(rook_empty_board_attack_bb, Some(rook_empty_board_attack_bb_no_edges));
            let all_bishop_block_masks: Vec<u64> = move_gen::generate_all_blocker_masks(bishop_empty_board_attack_bb, Some(bishop_empty_board_attack_bb_no_edges));
            //for rook
            find_working(sqr as u32, true, &mut rook_magics, &all_rook_block_masks,  &mut rng);
            //for bishop
            find_working(sqr as u32, false,  &mut bishop_magics, &all_bishop_block_masks, &mut rng);
            //now we can build the inner magic indexed vectors for this sqr with legal slides and push to outer vec
            println!("on sqr {}", sqr);
            let mut rook_lookup_vec: Vec<u64> = vec![0u64 ; 1 << ROOK_BITS[sqr]];
            for block_mask in all_rook_block_masks { //for rook
                let magic_idx: usize = ((block_mask.wrapping_mul(rook_magics[sqr])) >> ROOK_SHIFTS[sqr]) as usize;
                rook_lookup_vec[magic_idx] = naive_rook_sliding(sqr as u32, block_mask, true);
            }
            let mut bishop_lookup_vec: Vec<u64> = vec![0u64 ; 1 << BISHOP_BITS[sqr]];
            for block_mask in all_bishop_block_masks { //for bishop
                let magic_idx: usize = ((block_mask.wrapping_mul(bishop_magics[sqr])) >> BISHOP_SHIFTS[sqr]) as usize;
                bishop_lookup_vec[magic_idx] = naive_bishop_sliding(sqr as u32, block_mask, true);
            }
            rook_slide_bbs.push(rook_lookup_vec); //add this sqr to outer lookup vec
            bishop_slide_bbs.push(bishop_lookup_vec);
        }
        //now magics computed and 'legal' slide bbs filled magically indexed
        return Self { rook_magics, bishop_magics, rook_slide_bbs, bishop_slide_bbs }
    }
    ///For piece at **sqr** give blocker bitboard (ONLY RELEVANT BLOCKERS) and get corresponding magic idx with which to lookup from arr.
    ///Blocker bitboard has to be masked with no_edge bbs to contain only relevant blockers
    pub fn get_magic_idx(&self, sqr: usize, rel_blockers: u64, rook: bool) -> usize {
        if rook {
            return ((rel_blockers.wrapping_mul(self.rook_magics[sqr])) >> ROOK_SHIFTS[sqr]) as usize;
        } else {
            return ((rel_blockers.wrapping_mul(self.bishop_magics[sqr])) >> BISHOP_SHIFTS[sqr]) as usize;
        }
    }

}

///Find working magic number for **sqr** and update result to collections.
/// **all_block_masks** are RELEVANT blocker masks
fn find_working(sqr: u32, rook: bool, magic_arr: &mut [u64 ; 64], all_block_masks: &Vec<u64>, rng: &mut ThreadRng) {
    let piece_idx: usize;
    let bits_needed: u32;
    if rook {piece_idx = 3; bits_needed = ROOK_BITS[sqr as usize];} else {piece_idx = 2; bits_needed = BISHOP_BITS[sqr as usize];}
    let shift_amount: u32 = 64 - bits_needed;
    //now try different random magic numbers until find one that works for all masks
    let mut iterations = 0;
    let used_capacity: usize = 1 << bits_needed; //for **used** vec
    loop {
        iterations += 1;
        let mut collided: bool = false;
        let mut used: Vec<bool> = vec![false; used_capacity]; //used indices, if collision magic number doesn't work
        let magic: u64 = gen_random_magic(rng);
        for block_mask in all_block_masks {
            let magic_idx: usize = (((*block_mask).wrapping_mul(magic)) >> shift_amount) as usize;
            if used[magic_idx] { //collision, this magic doesn't work
                collided = true;
                break;
            }
            //no collision, mark this as used
            used[magic_idx] = true;
        }
        if !collided { //done, found working magic
            magic_arr[sqr as usize] = magic; //insert into magic arr
            println!("found magic with {} iterations", iterations);
            break;
        }
        if iterations > 1_000_000 {
            panic!("COULDNT FIND MAGIC, TIMED OUT");
        }
    }
    return;
}

///Get random magic number (the multiplier)
fn gen_random_magic(rng: &mut ThreadRng) -> u64 {
    return rng.next_u64() & rng.next_u64() & rng.next_u64();
}

///How many bits are used to index sqr at idx in magic computation
const ROOK_BITS: [u32 ; 64] = [
  12, 11, 11, 11, 11, 11, 11, 12,
  11, 10, 10, 10, 10, 10, 10, 11,
  11, 10, 10, 10, 10, 10, 10, 11,
  11, 10, 10, 10, 10, 10, 10, 11,
  11, 10, 10, 10, 10, 10, 10, 11,
  11, 10, 10, 10, 10, 10, 10, 11,
  11, 10, 10, 10, 10, 10, 10, 11,
  12, 11, 11, 11, 11, 11, 11, 12
];

const ROOK_SHIFTS: [u32 ; 64] = [
  52, 53, 53, 53, 53, 53, 53, 52,
  53, 54, 54, 54, 54, 54, 54, 53,
  53, 54, 54, 54, 54, 54, 54, 53,
  53, 54, 54, 54, 54, 54, 54, 53,
  53, 54, 54, 54, 54, 54, 54, 53,
  53, 54, 54, 54, 54, 54, 54, 53,
  53, 54, 54, 54, 54, 54, 54, 53,
  52, 53, 53, 53, 53, 53, 53, 52
];

const BISHOP_BITS: [u32 ; 64] = [
  6, 5, 5, 5, 5, 5, 5, 6,
  5, 5, 5, 5, 5, 5, 5, 5,
  5, 5, 7, 7, 7, 7, 5, 5,
  5, 5, 7, 9, 9, 7, 5, 5,
  5, 5, 7, 9, 9, 7, 5, 5,
  5, 5, 7, 7, 7, 7, 5, 5,
  5, 5, 5, 5, 5, 5, 5, 5,
  6, 5, 5, 5, 5, 5, 5, 6
];

const BISHOP_SHIFTS: [u32 ; 64] = [
  58, 59, 59, 59, 59, 59, 59, 58,
  59, 59, 59, 59, 59, 59, 59, 59,
  59, 59, 57, 57, 57, 57, 59, 59,
  59, 59, 57, 55, 55, 57, 59, 59,
  59, 59, 57, 55, 55, 57, 59, 59,
  59, 59, 57, 57, 57, 57, 59, 59,
  59, 59, 59, 59, 59, 59, 59, 59,
  58, 59, 59, 59, 59, 59, 59, 58
];


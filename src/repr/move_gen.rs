use std::collections::HashMap;

use crate::repr::_move;
use crate::repr::_move::*;
use crate::repr::board::*;
use crate::repr::magic_bb_loader::MagicBitboard;
use crate::repr::types::*;
use crate::repr::bitboard;

pub const KNIGHT_JUMPS: [(i32, i32); 8] = [(1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2)]; //in format (dx, dy), used for precomputing attack_bbs
pub const DIAG_STEPS: [(i32, i32); 4] = [(1, 1), (1, -1), (-1, -1), (-1, 1)];
pub const CARDINAL_STEPS: [(i32, i32); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];
const WS_CASTLING_GAP_BB: u64 = 96; //2^5 + 2^6
const WL_CASTLING_GAP_BB: u64 = 14; //2^1 + 2^2 + 2^3
const BS_CASTLING_GAP_BB: u64 = 6917529027641081856; //2^61 + 2^62
const BL_CASTLING_GAP_BB: u64 = 1008806316530991104; //2^57 + 2^58 + 2^59

pub struct MoveGen {
    pub attack_bbs: [[u64 ; 64] ; 12], //for each piece on each square the squares it attacks on an empty board. For pawns doesn't include forward moves, since they aren't attacked by pawns. Doesn't include en passant or castling either.
    pub rook_bbs_no_edges: [u64 ; 64], //slides on empty board without the last square on edge for each direction. Used for block masks since they are optimized by not including edges.
    pub bishop_bbs_no_edges: [u64 ; 64], //same but for bishops
    pub magic_bb: MagicBitboard //we hold the MagicBitboard struct privately here to avoid unneccessary passing extra handles
}

impl MoveGen {
    ///Initializes MoveGen. Simultaneously initializes magic_bb inside **magic_bb** option wrapping and optionally a testing structure inside **test_structure** wrapping to test slide dictionaries.
    pub fn init() -> Self {
        let mut attack_bbs: [[u64 ; 64] ; 12] = [[0 ; 64] ; 12];
        let mut rook_bbs_no_edges: [u64 ; 64] = [0 ; 64];
        let mut bishop_bbs_no_edges: [u64 ; 64] = [0 ; 64];
        //construct attack_bbs
        let mut x: u32 = 0;
        let mut y: u32 = 0;
        while y < 8 {
            while x < 8 {
                process_square(x, y, &mut attack_bbs, &mut rook_bbs_no_edges, &mut bishop_bbs_no_edges);
                x += 1;
            }
            x = 0;
            y += 1;
        }
        //initialize magic_bb struct, which contains final magically indexed precomputed slide_bbs
        let magic_bb: MagicBitboard = MagicBitboard::init_magic(&attack_bbs, &rook_bbs_no_edges, &bishop_bbs_no_edges); 
        return Self { attack_bbs, rook_bbs_no_edges, bishop_bbs_no_edges, magic_bb }
    }
    ///get 'legal' sliding moves at **sqr** with (relevant) blockers **rel_blockers**.
    /// if **cardinal** then adds rook moves
    /// if **diag** then adds bishop moves
    /// for queen moves both flags are set
    pub fn get_sliding_for(&self, sqr: usize, rel_blockers: u64, cardinal: bool, diag: bool) -> u64 {
        let mut res: u64 = 0;
        if cardinal {
            res |= self.magic_bb.rook_slide_bbs[sqr][self.magic_bb.get_magic_idx(sqr, rel_blockers, true)];
        }
        if diag {
            res |= self.magic_bb.bishop_slide_bbs[sqr][self.magic_bb.get_magic_idx(sqr, rel_blockers, false)];
        }
        return res;
    }
}

///Compute and add attacking bitboard for all pieces at (x, y)
fn process_square(x: u32, y: u32, attack_bbs: &mut[[u64 ; 64] ; 12], rook_bbs_no_edges: &mut[u64 ; 64], bishop_bbs_no_edges: &mut[u64 ; 64]) {
    let sqr_idx: u32 = x % 8 + y * 8;
    //with pawns moves for black the same as white but flipped and mirrored 
    //for all other pieces attacking squares are identical for white and black
    //1. pawn attacks, flip bb for black corresponding (at different idx)
    let white_pawn_bb: u64 = pawn_attacks_white_for(sqr_idx);
    let black_pawn_bb: u64 = white_pawn_bb.swap_bytes();
    attack_bbs[0][sqr_idx as usize] = white_pawn_bb;
    attack_bbs[6][(x % 8 + (7 - y) * 8) as usize] = black_pawn_bb;
    //2. knight attacks
    let mut bb_knight: u64 = 0;
    for jump in KNIGHT_JUMPS {
        let target: (i32, i32) = (x as i32 + jump.0, y as i32 + jump.1);
        if target.0 >= 0 && target.0 < 8 && target.1 >= 0 && target.1 < 8 { //add if in bounds of board
            bitboard::set_square(&mut bb_knight, (target.0 % 8 + target.1 * 8) as u32);
        }
    }
    attack_bbs[1][sqr_idx as usize] = bb_knight;
    attack_bbs[7][sqr_idx as usize] = bb_knight;
    //3. bishop attacks
    let mut bishop_bb: u64 = 0;
    let mut king_bb: u64 = 0; //simultaneously compute king diagonals (when 1 step taken)
    //in each diag dir take steps as long as on board
    for diag_step in DIAG_STEPS {
        let mut cur_target = (x as i32 + diag_step.0, y as i32 + diag_step.1);
        let mut steps_taken: u32 = 1;
        while cur_target.0 >= 0 && cur_target.0 < 8 && cur_target.1 >= 0 && cur_target.1 < 8 {
            bitboard::set_square(&mut bishop_bb, (cur_target.0 % 8 + cur_target.1 * 8) as u32);
            if steps_taken == 1 { //set king bb
                bitboard::set_square(&mut king_bb, (cur_target.0 % 8 + cur_target.1 * 8) as u32);
            }
            cur_target.0 += diag_step.0;
            cur_target.1 += diag_step.1;
            steps_taken += 1;
        }
    }
    attack_bbs[2][sqr_idx as usize] = bishop_bb;
    attack_bbs[8][sqr_idx as usize] = bishop_bb;
    //4. rook attacks
    let mut rook_bb: u64 = 0;
    //in each diag dir take steps as long as on board, simultaneously get king cardinals
    for cardinal_step in CARDINAL_STEPS {
        let mut cur_target = (x as i32 + cardinal_step.0, y as i32 + cardinal_step.1);
        let mut steps_taken: u32 = 1;
        while cur_target.0 >= 0 && cur_target.0 < 8 && cur_target.1 >= 0 && cur_target.1 < 8 {
            bitboard::set_square(&mut rook_bb, (cur_target.0 % 8 + cur_target.1 * 8) as u32);
            if steps_taken == 1 { //set king bb
                bitboard::set_square(&mut king_bb, (cur_target.0 % 8 + cur_target.1 * 8) as u32);
            }
            cur_target.0 += cardinal_step.0;
            cur_target.1 += cardinal_step.1;
            steps_taken += 1;
        }
    }
    attack_bbs[3][sqr_idx as usize] = rook_bb;
    attack_bbs[9][sqr_idx as usize] = rook_bb;
    //5. queen attacks: bishop attacks | rook attacks
    attack_bbs[4][sqr_idx as usize] = rook_bb | bishop_bb;
    attack_bbs[10][sqr_idx as usize] = rook_bb | bishop_bb;
    //6. king attacks: computed already inside rook and bishop
    attack_bbs[5][sqr_idx as usize] = king_bb;
    attack_bbs[11][sqr_idx as usize] = king_bb;
    //lastly add no-edge variation for rooks and bishops
    rook_bbs_no_edges[sqr_idx as usize] = naive_rook_sliding(sqr_idx, 0, false);
    bishop_bbs_no_edges[sqr_idx as usize] = naive_bishop_sliding(sqr_idx, 0, false);
}

///Precomputes rook sliding moves at square (x, y) for every possible blocking mask to lookup.
///Adds to the dictionary.
fn process_rook_sliding(x: u32, y: u32, lookup: &mut HashMap<(u32, u64), u64>, attack_bb: u64, attack_bb_no_edges: u64) {
    let sqr_idx: u32 = x % 8 + y * 8;
    let all_masks: Vec<u64> = generate_all_blocker_masks(attack_bb, None); //we want edges included here
    //iterate through each mask and compute naive sliding moves for it to dictionary
    for mask in all_masks {
        //when we insert we REMOVE edges from the key but KEEP the edges in the value
        lookup.insert((sqr_idx, mask & attack_bb_no_edges), naive_rook_sliding(sqr_idx, mask, true));
    }
}

///Returns all variations of blocker masks from **full_attack_bb*.
///**attack_bb_no_edges** must be defined if the last square before edge is wished to not be included
pub fn generate_all_blocker_masks(mut full_attack_bb: u64, attack_bb_no_edges: Option<u64>) -> Vec<u64> {
    let include_edges: bool = attack_bb_no_edges.is_none();
    let mut res: Vec<u64> = Vec::new();
    if !include_edges {
        full_attack_bb &= attack_bb_no_edges.expect("Checked that Some but was None.")
    }
    let mut cur_block_mask: u64 = full_attack_bb;
    //iterate through all subsets of blocking mask and add to res 
    loop {
        if cur_block_mask == 0 {
            if include_edges {res.push(cur_block_mask)} else {res.push(cur_block_mask & attack_bb_no_edges.unwrap())}
            break;
        }
        if include_edges {res.push(cur_block_mask)} else {res.push(cur_block_mask & attack_bb_no_edges.unwrap())}
        cur_block_mask = (cur_block_mask - 1) & full_attack_bb;
    }
    return res;
}

///Returns a bitboard of possible rook sliding moves at sqr with given blockers computed naively. 
/// The first blocker found is included in the possible slide moves. (Assume that all blockers enemy. This assumption can be relieved later)
/// take_edge flag controls if the possible edge squares are included in the bb
/// **blockers** can be either strictly relevant or irrelevant inclusive, since by definition it doesn't matter
pub fn naive_rook_sliding(sqr: u32, blockers: u64, include_edge: bool) -> u64 {
    let mut possible_slides: u64 = 0;
    //go right
    possible_slides |= slide_to_dir(sqr, |x| x + 1, H_FILE, blockers, include_edge);
    //go left
    possible_slides |= slide_to_dir(sqr, |x| x - 1, A_FILE, blockers, include_edge);
    //go down
    possible_slides |= slide_to_dir(sqr, |x| x - 8, RANK_1, blockers, include_edge);
    //go up
    possible_slides |= slide_to_dir(sqr, |x| x + 8, RANK_8, blockers, include_edge);
    return possible_slides;
}
///Return bb with all possible bishop slides on **sqr** with given **blockers**. **include_edge** flag controls if squares before edge are taken.
pub fn naive_bishop_sliding(sqr: u32, blockers: u64, include_edge: bool) -> u64 {
    let mut possible_slides: u64 = 0;
    //go NE
    possible_slides |= slide_to_dir(sqr, |x| x + 9, H_FILE | RANK_8, blockers, include_edge);
    //go NW
    possible_slides |= slide_to_dir(sqr, |x| x + 7, A_FILE | RANK_8, blockers, include_edge);
    //go SE
    possible_slides |= slide_to_dir(sqr, |x| x - 7, H_FILE | RANK_1, blockers, include_edge);
    //go SW
    possible_slides |= slide_to_dir(sqr, |x| x - 9, A_FILE | RANK_1, blockers, include_edge);
    return possible_slides;
}

///generalize stepping to some direction with end edge and blockers to find possible sliding squares returned in bb
fn slide_to_dir(sqr: u32, apply_step_f: fn(u32) -> u32, end_sqr_bb: u64, blockers: u64, include_edge: bool) -> u64 {
    let mut res: u64 = 0;
    let mut cur_sqr: u32 = sqr;
    if !bitboard::contains_square(end_sqr_bb, cur_sqr) { //if not already on edge (end_sqr_bb)
        cur_sqr = apply_step_f(cur_sqr);
        while !bitboard::contains_square(blockers, cur_sqr) && !bitboard::contains_square(end_sqr_bb, cur_sqr) { //stop when blocker or on end edge (end_sqr_bb)
            res |= 1 << cur_sqr;
            cur_sqr = apply_step_f(cur_sqr);
        }
        //now add the blocker or edge if needed
        let on_edge: bool = bitboard::contains_square(EDGES, cur_sqr);
        if (on_edge && include_edge) || !on_edge { //always take blocker if not on edge
            res |= 1 << cur_sqr;
        }
    }
    return res;
}

///Used for initialization of attacking bitboards
///Computes the attacking squares for **sqr** from white's perspective
fn pawn_attacks_white_for(sqr: u32) -> u64 {
    let non_accessible_ranks: u64 = (RANK_2 >> 8) | (RANK_7 << 8); //first and last rank non-accessible
    if bitboard::contains_square(non_accessible_ranks, sqr) { 
        return 0;
    }
    let mut res: u64 = 0;
    //add left attacks, removing overflow to H file
    res |= bitboard::diff(((1u128 << sqr) << 7) as u64, H_FILE);
    //add right attacks, removing overflow to A file
    res |= bitboard::diff(((1u128 << sqr) << 9) as u64, A_FILE);
    return res;
}


///Adds LEGAL castling moves for **mover** to move vector **move_vec**.
fn add_castling(board: &Board, mover: Color, move_vec: &mut Vec<u32>) {
    if mover.is_white() {
        let occupation_bb: u64 = board.white_occupation | board.black_occupation;
        if !board.mover_in_check { //king not checked, necessary for castling
            if board.ws && WS_CASTLING_GAP_BB & occupation_bb == 0 { //short is legal
            move_vec.push(WHITE_SHORT);
        }
            if board.wl && WL_CASTLING_GAP_BB & occupation_bb == 0 { //long is legal
                move_vec.push(WHITE_SHORT);
            }
        }
    } else {
        let occupation_bb: u64 = board.white_occupation | board.black_occupation;
        if !board.mover_in_check {
            if board.bs && BS_CASTLING_GAP_BB & occupation_bb == 0 { //short is legal
                move_vec.push(BLACK_SHORT);
            }
            if board.bl && BL_CASTLING_GAP_BB & occupation_bb == 0 { //long is legal
                move_vec.push(BLACK_LONG);
            }
        }
    }
}
/* 
///Adds pseudolegal moves for **piece** at **from** to move vector **move_vec**.
pub fn pseudolegal_for(from: u32, piece: u32, mover: Color, move_gen: &MoveGen, board: &Board, move_vec: &mut Vec<u32>) {
    match piece {
        W_PAWN | B_PAWN => pseudolegal_pawn(from, mover, board, move_gen, move_vec),
        W_KNIGHT | B_KNIGHT =>  pseudolegal_knight(from, mover, board, move_gen, move_vec),
        W_BISHOP | B_BISHOP => 
    }
}
 */
/// Add all pseudolegal knight moves for knight on square **from** and color **mover** on **board** to **move_vec** .
fn pseudolegal_knight(from: u32, mover: Color, board: &Board, move_gen: &MoveGen, move_vec: &mut Vec<u32>) {
    let own_occupied: u64;
    let knight_idx: u32;
    if mover.is_white() {own_occupied = board.white_occupation; knight_idx = 1;} else {own_occupied = board.black_occupation; knight_idx = 7;};
    //add attacking moves
    let mut characteristics: u64 = move_gen.attack_bbs[1][from as usize]; //bitboard (same for black and white)
    while characteristics != 0 { //check each characteristic jump
        let attack_sqr: u32 = bitboard::pop_lsb(&mut characteristics);
        if !bitboard::contains_square(own_occupied, attack_sqr) { //is pseudolegal (not eating own piece)
            let takes: Option<u32> = board.lift_piece_type_at(attack_sqr, mover.opposite());
            move_vec.push(_move::create(from, attack_sqr, takes, mover, knight_idx));
        }
    }
    return;
}


/// Add all pseudolegal pawn moves for pawn on square **from** and color **mover** on **board** to **move_vec** . NO EN PASSANT
pub fn pseudolegal_pawn(from: u32, mover: Color, board: &Board, move_gen: &MoveGen, move_vec: &mut Vec<u32>)  {
    //following are relative to color:
    let is_promotion: bool;
    let forward: u32;
    let forward2: u32;
    let PAWN_START_RANK: u64;
    let pawn_piece_idx: usize;
    let enemy_occupied: u64;
    if mover.is_white() { 
        is_promotion = bitboard::contains_square(RANK_7, from);
        forward = from + 8; forward2 = from + 16; PAWN_START_RANK = RANK_2; pawn_piece_idx = 0; enemy_occupied = board.black_occupation;
    } else { //for black
        is_promotion = bitboard::contains_square(RANK_2, from);
        forward = from - 8; forward2 = from - 16; PAWN_START_RANK = RANK_7; pawn_piece_idx = 6; enemy_occupied = board.white_occupation;
    }
    //add attacking moves
    let mut characteristic_attacks: u64 = move_gen.attack_bbs[pawn_piece_idx][from as usize]; //bitboard
    while characteristic_attacks != 0 { //check each attack
        let attack_sqr: u32 = bitboard::pop_lsb(&mut characteristic_attacks);
        if bitboard::contains_square(enemy_occupied, attack_sqr) { //is pseudolegal
            let takes: Option<u32> = Some(board.get_piece_type_at(attack_sqr, mover.opposite()));
            if is_promotion {
                add_all_promotions(from, attack_sqr, takes, mover, move_vec);
            } else {
                move_vec.push(_move::create(from, attack_sqr, takes, mover, pawn_piece_idx as u32));
            }
        }
    }
    //now check forward moves
    //can go forward 1 if no piece there
    if !board.is_occupied(forward) {
        if is_promotion {
            _move::add_all_promotions(from, forward, None, mover, move_vec);
        } else {
            move_vec.push(_move::create(from, forward, None, mover, pawn_piece_idx as u32))
        }
        //can go forward 2 if also no piece on forward2 and pawn on PAWN_START_RANK
        if bitboard::contains_square(PAWN_START_RANK, from) && !board.is_occupied(forward2) {
            move_vec.push(_move::create(from, forward2, None, mover, pawn_piece_idx as u32))
        }
    }
    //NO EN PASSANT FROM THIS
    return;
}

///Test some critical temporary data computed on initialization.  
pub struct TestMoveGen {
    pub dictionary_rook_slide_bbs: HashMap<(u32, u64), u64>,
    pub dictionary_bishop_slide_bbs: HashMap<(u32, u64), u64>
}
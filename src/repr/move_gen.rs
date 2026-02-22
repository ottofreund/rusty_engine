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

///Uses **magic_bb** handle for precomputed slide moves.
pub struct MoveGen {
    pub attack_bbs: [[u64 ; 64] ; 12], //empty board attack bbs, for pawns doesn't include forward moves, since they aren't attacked by pawns. Doesn't include en passant or castling either.
    pub rook_bbs_no_edges: [u64 ; 64], //slides on empty board without the last square on edge for each direction. Used for block masks since they are optimized by not including edges.
    pub bishop_bbs_no_edges: [u64 ; 64],
    pub magic_bb: MagicBitboard,
}

impl MoveGen {
    ///Necessarily also inits magicbbs
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
        let magic_bb: MagicBitboard = MagicBitboard::init_magic(&attack_bbs, &rook_bbs_no_edges, &bishop_bbs_no_edges, false); 
        return Self { attack_bbs, rook_bbs_no_edges, bishop_bbs_no_edges, magic_bb }
    }

    ///Called once upon arriving to a new position. <br>
    ///Not called when reverting move, since just fetched from stack.
    pub fn get_all_legal(&self, board: &Board, mover: Color) -> Vec<u32> {
        let mut res: Vec<u32> = Vec::new();
        for mov in self.get_all_pseudolegal(board, mover) {
            if self.pseudolegal_is_legal(mov, board, mover) {
                //add taken piece idx to move (if eating) now, since it is necessary
                let m: u32;
                if _move::is_en_passant(mov) {
                    if mover.is_white() {
                        m = _move::with_eaten_piece(mov, 6);
                    } else {
                        m = mov; //white pawn is 0 so do nothing
                    }   
                } else if _move::is_eating(mov) {
                    let eaten_piece: u32 = board.get_piece_type_at(_move::get_target(mov), mover.opposite());
                    m = _move::with_eaten_piece(mov, eaten_piece);
                } else {
                    m = mov;
                }
                res.push(m);
            }
        }
        return res;
    }

    ///Edge cases: For en passant check pin edge case, for king check not moving to attacked squares
    pub fn pseudolegal_is_legal(&self, mov: u32, board: &Board, mover: Color) -> bool {
        let init: u32 = _move::get_init(mov);
        let moved_piece: u32 = _move::get_moved_piece(mov);
        let opponent_attacked: u64;
        let mover_king_piece_idx: u32;
        let mover_pinned: u64;
        let mover_pinned_restrictions: u64;
        if mover.is_white() {
            opponent_attacked = board.black_attacks; mover_king_piece_idx = 5; 
            mover_pinned = board.white_pinned; mover_pinned_restrictions = board.white_pinned_restrictions[init as usize];
        } 
        else {
            opponent_attacked = board.white_attacks; mover_king_piece_idx = 11;
            mover_pinned = board.black_pinned; mover_pinned_restrictions = board.black_pinned_restrictions[init as usize];
        }
        
        if board.nof_checkers > 0 {
            let moved_king: bool = moved_piece == W_KING || moved_piece == B_KING;
            if board.nof_checkers == 1 {
                let target: u32 = _move::get_target(mov);
                let blocked_check: bool = bitboard::contains_square(board.check_block_sqrs, target);
                //if one checker, can either move king to safe square or block (including eat)
                if !moved_king && !blocked_check {
                    return false; 
                } //if moved king, next section checks that king's target is legal
            } else if board.nof_checkers == 2 {
                //if two checkers, only chance is to move king to safe square
                if !moved_king {
                    return false; 
                } //in next section we check if the king's target square is legal (not attacked)
            } else { //shouldn't be > 2
                panic!("Counted 3 or more checkers, which should be impossible.")
            }
            
        }

        if _move::is_en_passant(mov) && board.get_king_sqr_idx(mover) / 8 == init / 8 {
            //check edge case where both pawns leave rank exposing pin on same rank king
            let opponent_horizontal_sliding: u64;
            let ep_rank: u64;
            if mover.is_white() {
                opponent_horizontal_sliding = board.pieces[9] | board.pieces[10];
                ep_rank = RANKS[4];
            } else {
                opponent_horizontal_sliding = board.pieces[3] | board.pieces[4];
                ep_rank = RANKS[3];
            }
            if ep_rank & opponent_horizontal_sliding > 0 {
                let scan_right: bool = board.get_king_sqr_idx(mover) < init;
                let ep_dir_right: bool = _move::get_target(mov) % 8 > init % 8;
                let scan_start_sqr: u32;
                let apply_step_f: fn(u32) -> u32;
                let end_sqr_bb: u64;
                if scan_right {
                    apply_step_f = |x: u32| x + 1;
                    end_sqr_bb = FILES[7];
                    if ep_dir_right {
                        scan_start_sqr = init + 1;
                    } else {
                        scan_start_sqr = init;
                    }
                } else {
                    apply_step_f = |x: u32| x - 1;
                    end_sqr_bb = FILES[0];
                    if ep_dir_right {
                        scan_start_sqr = init;
                    } else {
                        scan_start_sqr = init - 1;
                    }
                }
                let slide_bb: u64 = slide_to_dir(scan_start_sqr, apply_step_f, end_sqr_bb, board.total_occupation(), true);
                if slide_bb & opponent_horizontal_sliding > 0 {
                    return false;
                }
            }
        } else if moved_piece == mover_king_piece_idx {
            let target: u32 = _move::get_target(mov);
            if bitboard::contains_square(opponent_attacked, target)
                || bitboard::contains_square(board.meta_attacks, target) {
                return false;
            } else {
                return true;
            }
        }
        //now pins
        if bitboard::contains_square(mover_pinned, init) {
            let target: u32 = _move::get_target(mov);
            if !bitboard::contains_square(mover_pinned_restrictions, target) {
                return false;
            }
        }
        return true;
    }

    ///Also updates board.nof_checkers for non-sliding pieces (sliding checkers found at pinned computation) 
    pub fn get_all_pseudolegal(&self, board: &Board, mover: Color) -> Vec<u32> {
        let mut res: Vec<u32> = Vec::new();
        let mut i: usize;
        let e: usize;
        if mover.is_white() {i = 0; e = 6;} else {i = 6; e = 12;}
        while i < e {
            let mut piece_bb: u64 = board.pieces[i];
            while piece_bb != 0 {
                let piece_idx: u32 = bitboard::pop_lsb(&mut piece_bb);
                self.pseudolegal_for(piece_idx, i as u32, mover, board, &mut res, false, false);
            }
            i += 1;
        }
        add_en_passant(board, mover, &mut res);
        add_castling(board, mover, &mut res);
        return res;
    }

    ///Adds pseudolegal moves for **piece** at **from** to move vector **move_vec**. <br>
    ///1. If pawn, handle separately <br>
    ///2. Get target squares (including eating own pieces), sliding gen or simply attack_bb <br>
    ///3. If !**keep_protected**, remove "eating own piece" moves by binary ANDing with !own_occupation <br>
    ///4. If **targets_only** just return targets bitboard.
    ///5. Else Pop-lsb 1-by-1 and make move and add to **move_vec** until none left.
    pub fn pseudolegal_for(&self, from: u32, piece: u32, mover: Color, board: &Board, move_vec: &mut Vec<u32>, keep_protected: bool, targets_only: bool) -> u64 {
        if piece == W_PAWN || piece == B_PAWN {
            //no en passant from this
            pseudolegal_pawn(from, mover, board, self, move_vec);
            return 0;
        }

        let mut targets: u64 = match piece {
            W_KNIGHT | B_KNIGHT =>  self.attack_bbs[1][from as usize],
            W_BISHOP | B_BISHOP =>  {
                self.get_sliding_for(from as usize, self.get_relevant_blockers(from as usize, bitboard::with_clear_square(board.total_occupation(), from), false), false)
            },
            W_ROOK | B_ROOK =>  {
                self.get_sliding_for(from as usize, self.get_relevant_blockers(from as usize, bitboard::with_clear_square(board.total_occupation(), from), true), true)
            },
            W_QUEEN | B_QUEEN =>  {
                let blockers: u64 = bitboard::with_clear_square(board.total_occupation(), from);
                self.get_sliding_for(from as usize, self.get_relevant_blockers(from as usize, blockers, true), true)
                    |
                self.get_sliding_for(from as usize, self.get_relevant_blockers(from as usize, blockers, false), false)
            },
            W_KING | B_KING => self.attack_bbs[5][from as usize],
            _ => panic!("Couldn't match piece in pseudolegal_for. Reached unreachable case.")    
        };
        //3.
        let opponent_occupied: u64;
        if mover.is_white() {
            if !keep_protected {
                targets &= !board.white_occupation;
            }
            opponent_occupied = board.black_occupation;
        } else {
            if !keep_protected {
                targets &= !board.black_occupation;
            }
            opponent_occupied = board.white_occupation;
        }
        if targets_only { //4.
            return targets;
        } else { //5.
            while targets != 0 { 
                let target_sqr: u32 = bitboard::pop_lsb(&mut targets);
                let is_take: bool = bitboard::contains_square(opponent_occupied, target_sqr);
                move_vec.push(_move::create(from, target_sqr, is_take, mover, piece));
            }
            return 0;
        }
    }

    ///get pseudolegal sliding moves at **sqr** with **rel_blockers**. <br/><br>
    /// for queen moves this is called twice, once with **rook** true and once with **rook** false <br/><br>
    /// does NOT remove own pieces here yet since defended pieces are also of interest.
    pub fn get_sliding_for(&self, sqr: usize, rel_blockers: u64, rook: bool) -> u64 {
        if rook {
            return self.magic_bb.rook_slide_bbs[sqr][self.magic_bb.get_magic_idx(sqr, rel_blockers, true)];
        } else {
            return self.magic_bb.bishop_slide_bbs[sqr][self.magic_bb.get_magic_idx(sqr, rel_blockers, false)];
        }
    }

    ///Get all relevant blockers for blocker mask. This means excluding edge squares on edges that sqr is not on.<br/>
    /// scan_cardinal for rook, scan_diag for bishop, both for queen <br/>
    /// Very good constant time complexity due to memoization of no-edge bitboards
    pub fn get_relevant_blockers(&self, sqr: usize, blockers: u64, cardinal: bool) -> u64 {
        if cardinal {
            return blockers & self.rook_bbs_no_edges[sqr];
        } else {
            return blockers & self.bishop_bbs_no_edges[sqr];
        }
    }

    ///Computes pins on **board** for **side** into its fields ({white/black}_pinned, {white_black}_pinned_restrictions) so that this only needs to be called once after making/unmaking move and after having all needed info in those fields.
    /// 1. Binary and rook empty board attack bb from king's square with opponent's combined (ORed) queen and rook occupation bbs. Call this the potential pinners bb.
    /// 2. If potential pinners empty stop and go to 1. but for diagonals.
    /// 3. Get rook **blocking** attack bb from king's sqr with blockers being opponent's combined (ORed) queen and rook occupation bbs. Call this the RPP bitboard (relevant potential pinned)
    /// 4. For each potential pinner:<br/>
    ///     4.1. Get the potential pinner sqr_idx from lsb pop <br/>
    ///     4.2. Binary AND RPP with rook **blocking** attack bb from **sqr_idx** with king as the only blocker.  <br/>
    ///          Untoggle sqr_idx from this to get specific RPP. <br/>
    ///     4.3  Binary AND specific RPP with (white_occupation | black_occupation) and get count of ones in result bb.<br/>
    ///     4.4  If count_ones == 1 then we have pinner: <br/>
    ///         4.4.1. Pop the idx of pinner with pop_lsb and set board.pinned bit at idx. <br/>
    ///         4.4.2. Binary OR board.pinned_restrictions[idx] with specific RPP to add this restriction <br/>
    ///         Else no pins for this potential pinner, continue <br/>
    /// 5. Do same starting from 1. but for diagonals (bishop)
    pub fn compute_pinned(&self, board: &mut Board, side: Color) {
        //first reset bitboards containing previous pinned info
        board.white_pinned = 0;
        board.black_pinned = 0;
        board.white_pinned_restrictions = [0 ; 64];
        board.black_pinned_restrictions = [0 ; 64];
        board.meta_attacks = 0;
        //compute new pins
        self.pinned_for_specified(false, board, side);
        self.pinned_for_specified(true, board, side);
    }
    ///Perform described algorithm for diagonals or cardinals according to **diag** flag.
    fn pinned_for_specified(&self, diag: bool, board: &mut Board, side: Color) {
        //1.
        let opponent_sliding: u64; //opponent's queen, rook || bishop occupation
        let empty_sliding_from_king: u64; //empty sliding for rook/bishop from king's square
        let king_sqr_idx: usize = board.get_king_sqr_idx(side) as usize;
        //set color/direction specific vars
        if side.is_white() {
            if diag {
                opponent_sliding = board.pieces[10] | board.pieces[8];
                empty_sliding_from_king = self.attack_bbs[2][king_sqr_idx];
            } else {
                opponent_sliding = board.pieces[10] | board.pieces[9];
                empty_sliding_from_king = self.attack_bbs[3][king_sqr_idx];
            }       
        } else {
            if diag {
                opponent_sliding = board.pieces[4] | board.pieces[2];
                empty_sliding_from_king = self.attack_bbs[2][king_sqr_idx];
            } else {
                opponent_sliding = board.pieces[4] | board.pieces[3];
                empty_sliding_from_king = self.attack_bbs[3][king_sqr_idx];
            }       
        }
        
        let mut potential_pinners: u64 = opponent_sliding & empty_sliding_from_king;
        //2.
        if potential_pinners == 0 { return }; //no potential pinners
        //3.
        let rpp: u64 = self.get_sliding_for(king_sqr_idx, self.get_relevant_blockers(king_sqr_idx, opponent_sliding, !diag), !diag);
        let total_occupation: u64 = board.white_occupation | board.black_occupation;
        //4.
        while potential_pinners != 0 {
            //4.1
            let pp_sqr_idx: usize = bitboard::pop_lsb(&mut potential_pinners) as usize; //potential pinner sqr idx
            //4.2
            let specific_rpp: u64 = rpp & self.get_sliding_for(pp_sqr_idx, 1 << king_sqr_idx, !diag);
            //4.3
            let occupied_rpp: u64 = specific_rpp & total_occupation;
            //4.4
            if occupied_rpp.count_ones() == 1 { //we have pinner
                let pinner_idx: u32 = occupied_rpp.trailing_zeros(); //4.4.1
                //4.4.2
                if side.is_white() {
                    bitboard::set_square(&mut board.white_pinned, pinner_idx);
                    board.white_pinned_restrictions[pinner_idx as usize] |= bitboard::with_set_square(specific_rpp, pp_sqr_idx as u32); //can also eat pinner;
                } else {
                    bitboard::set_square(&mut board.black_pinned, pinner_idx);
                    board.black_pinned_restrictions[pinner_idx as usize] |= bitboard::with_set_square(specific_rpp, pp_sqr_idx as u32); //can also eat pinner;
                }
            } else if occupied_rpp.count_ones() == 0 { //we have a sliding checker, set the check_block_sqrs
                board.check_block_sqrs |= bitboard::with_set_square(specific_rpp, pp_sqr_idx as u32); //can also eat checker so set pp_sqr_idx
                //with sliding check we have a meta-attack behind king
                let empty_board_slides: u64;
                if diag { empty_board_slides = self.attack_bbs[2][pp_sqr_idx]; } else { empty_board_slides = self.attack_bbs[3][pp_sqr_idx]; }
                board.meta_attacks |= self.attack_bbs[5][king_sqr_idx] & empty_board_slides & !specific_rpp & !(1 << king_sqr_idx);
            }
        }
    }

    ///attacked squares change after moving piece. <br>
    ///we call this after moving to get the updated attacked squares <br>
    ///this means we compute targets twice, but the alternative implementations seem even worse <br><br>
    ///given we compute this, we also easily get nof_checkers
    pub fn compute_attacked(&self, board: &mut Board, side: Color) -> u64 {
        let mut res: u64 = pawn_attacked(board, side);
        let opponent_king_sqr: u32 = board.get_king_sqr_idx(side.opposite());
        if bitboard::contains_square(res, opponent_king_sqr) {
            board.nof_checkers += 1; //pawn checks now covered
        }
        let mut i: usize;
        let e: usize;
        if side.is_white() {i = 1; e = 6;} else {i = 7; e = 12;} //no pawns
        while i < e {
            let mut piece_bb: u64 = board.pieces[i];
            while piece_bb != 0 {
                let piece_idx: u32 = bitboard::pop_lsb(&mut piece_bb);
                let targets: u64 = self.pseudolegal_for(piece_idx, i as u32, side, board, &mut Vec::new(), true, true);
                if bitboard::contains_square(targets, opponent_king_sqr) { //see if checker
                    board.nof_checkers += 1;
                    bitboard::set_square(&mut board.check_block_sqrs, piece_idx);
                }
                res |= targets;
            }
            i += 1;
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


///Returns all variations of blocker masks from **full_attack_bb**. <br/>
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

///Returns a bitboard of possible rook sliding moves at sqr with given blockers computed naively.  <br/>
/// The first blocker found is included in the possible slide moves. (Assume that all blockers enemy. This assumption can be relieved later) <br/>
/// take_edge flag controls if the possible edge squares are included in the bb  <br/>
/// **blockers** can be either strictly relevant or irrelevant inclusive, since by definition it doesn't matter  <br/>
pub fn naive_rook_sliding(sqr: u32, blockers: u64, include_edge: bool) -> u64 {
    let mut possible_slides: u64 = 0;
    //go right
    possible_slides |= slide_to_dir(sqr, |x| x + 1, FILES[7], blockers, include_edge);
    //go left
    possible_slides |= slide_to_dir(sqr, |x| x - 1, FILES[0], blockers, include_edge);
    //go down
    possible_slides |= slide_to_dir(sqr, |x| x - 8, RANKS[0], blockers, include_edge);
    //go up
    possible_slides |= slide_to_dir(sqr, |x| x + 8, RANKS[7], blockers, include_edge);
    return possible_slides;
}
///Return bb with all possible bishop slides on **sqr** with given **blockers**. **include_edge** flag controls if squares before edge are taken.
pub fn naive_bishop_sliding(sqr: u32, blockers: u64, include_edge: bool) -> u64 {
    let mut possible_slides: u64 = 0;
    //go NE
    possible_slides |= slide_to_dir(sqr, |x| x + 9, FILES[7] | RANKS[7], blockers, include_edge);
    //go NW
    possible_slides |= slide_to_dir(sqr, |x| x + 7, FILES[0] | RANKS[7], blockers, include_edge);
    //go SE
    possible_slides |= slide_to_dir(sqr, |x| x - 7, FILES[7] | RANKS[0], blockers, include_edge);
    //go SW
    possible_slides |= slide_to_dir(sqr, |x| x - 9, FILES[0] | RANKS[0], blockers, include_edge);
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

///Used for initialization of attacking bitboards  <br/>
///Computes the attacking squares for **sqr** from white's perspective
fn pawn_attacks_white_for(sqr: u32) -> u64 {
    let non_accessible_ranks: u64 = RANKS[0] | RANKS[7]; //first and last rank non-accessible
    if bitboard::contains_square(non_accessible_ranks, sqr) { 
        return 0;
    }
    let mut res: u64 = 0;
    //add left attacks, removing overflow to H file
    res |= bitboard::diff(((1u128 << sqr) << 7) as u64, FILES[7]);
    //add right attacks, removing overflow to A file
    res |= bitboard::diff(((1u128 << sqr) << 9) as u64, FILES[0]);
    return res;
}

///Adds LEGAL castling moves for **mover** to **move_vec**.
fn add_castling(board: &Board, mover: Color, move_vec: &mut Vec<u32>) {
    if board.nof_checkers == 0 { //can't castle from check
        let preventing_bb: u64;
        let opponent_attacks: u64;
        if mover.is_white() {
            opponent_attacks = board.black_attacks;
            preventing_bb = board.white_occupation | board.black_occupation | opponent_attacks;
            if board.ws() && WS_CASTLING_GAP_BB & preventing_bb == 0 { //short is legal
                move_vec.push(WHITE_SHORT);
            }
            if board.wl() && WL_CASTLING_GAP_BB & preventing_bb == 0 { //long is legal
                move_vec.push(WHITE_SHORT);
            }
        } else {
            opponent_attacks = board.white_attacks;
            preventing_bb = board.white_occupation | board.black_occupation | opponent_attacks;
            if board.bs() && BS_CASTLING_GAP_BB & preventing_bb == 0 { //short is legal
                move_vec.push(BLACK_SHORT);
            }
            if board.bl() && BL_CASTLING_GAP_BB & preventing_bb == 0 { //long is legal
                move_vec.push(BLACK_LONG);
            }
        }
    } 
}

///Adds **pseudolegal** en passants to move_vec
fn add_en_passant(board: &Board, mover: Color, move_vec: &mut Vec<u32>) {
    if let Some(ep_square) = board.ep_square {
        let mover_pawns: u64;
        let l_sqr: u32;
        let r_sqr: u32;
        let pawn_piece_idx: u32;
        if mover.is_white() {
            mover_pawns = board.pieces[0]; l_sqr = ep_square - 9; r_sqr = ep_square - 7; pawn_piece_idx = 0;
        } else {
            mover_pawns = board.pieces[6]; l_sqr = ep_square + 7; r_sqr = ep_square + 9; pawn_piece_idx = 6;
        }
        if bitboard::contains_square(mover_pawns, l_sqr) {
            move_vec.push(_move::create_en_passant(l_sqr, ep_square, mover, pawn_piece_idx))
        }
        if bitboard::contains_square(mover_pawns, r_sqr) {
            move_vec.push(_move::create_en_passant(r_sqr, ep_square, mover, pawn_piece_idx))
        }
    }
}

/// Add all pseudolegal pawn moves for pawn on square **from** and color **mover** on **board** to **move_vec** . <br>
/// 
/// NO EN PASSANT
pub fn pseudolegal_pawn(from: u32, mover: Color, board: &Board, move_gen: &MoveGen, move_vec: &mut Vec<u32>)  {
    //following are relative to color:
    let is_promotion: bool;
    let forward: u32;
    let forward2: u32;
    let PAWN_START_RANK: u64;
    let pawn_piece_idx: usize;
    let enemy_occupied: u64;
    if mover.is_white() { 
        is_promotion = bitboard::contains_square(RANKS[6], from);
        forward = from + 8; forward2 = from + 16; PAWN_START_RANK = RANKS[1]; pawn_piece_idx = 0; enemy_occupied = board.black_occupation;
    } else { //for black
        is_promotion = bitboard::contains_square(RANKS[1], from);
        forward = from - 8; forward2 = from - 16; PAWN_START_RANK = RANKS[6]; pawn_piece_idx = 6; enemy_occupied = board.white_occupation;
    }
    //add attacking moves
    let mut characteristic_attacks: u64 = move_gen.attack_bbs[pawn_piece_idx][from as usize]; //bitboard
    while characteristic_attacks != 0 { //check each attack
        let attack_sqr: u32 = bitboard::pop_lsb(&mut characteristic_attacks);
        if bitboard::contains_square(enemy_occupied, attack_sqr) { //is pseudolegal
            if is_promotion {
                add_all_promotions(from, attack_sqr, true, mover, move_vec);
            } else {
                move_vec.push(_move::create(from, attack_sqr, true, mover, pawn_piece_idx as u32));
            }
        }
    }
    //now check forward moves
    //can go forward 1 if no piece there
    if !board.is_occupied(forward) {
        if is_promotion {
            _move::add_all_promotions(from, forward, false, mover, move_vec);
        } else {
            move_vec.push(_move::create(from, forward, false, mover, pawn_piece_idx as u32))
        }
        //can go forward 2 if also no piece on forward2 and pawn on PAWN_START_RANK
        if bitboard::contains_square(PAWN_START_RANK, from) && !board.is_occupied(forward2) {
            move_vec.push(_move::create_double_push(from, forward2, mover, pawn_piece_idx as u32))
        }
    }
    //NO EN PASSANT FROM THIS
    return;
}

///All pawn attacking moves for **side**
pub fn pawn_attacked(board: &Board, side: Color) -> u64 {
    let pawn_occupation: u64;
    let left_attack_shift: u32;
    let right_attack_shift: u32;
    let shift_op: fn(u64, u32) -> u64;
    let mut res: u64 = 0;
    if side.is_white() {
        pawn_occupation = board.pieces[0]; left_attack_shift = 7; right_attack_shift = 9; shift_op = |x, n| x << n;
    } else {
        pawn_occupation = board.pieces[6]; left_attack_shift = 9; right_attack_shift = 7; shift_op = |x, n| x >> n;
    };
    res |= shift_op(pawn_occupation & !FILES[0], left_attack_shift);
    res |= shift_op(pawn_occupation & !FILES[7], right_attack_shift);
    return res;
}
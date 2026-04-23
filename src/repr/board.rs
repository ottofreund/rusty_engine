use crate::repr::{position::MOVE_ARR_SIZE, move_gen::MoveGen, types::{B_KING, BLACK, W_KING, WHITE, opposite_turn}};


pub const FILES: [u64 ; 8] = [
    72340172838076673, 144680345676153346, 289360691352306692, 578721382704613384, 1157442765409226768, 2314885530818453536, 4629771061636907072, 0x8080808080808080
];
pub const RANKS: [u64 ; 8] = [
    0xFF, 65280, 0xFF0000, 0xFF000000, 0xFF00000000, 0xFF0000000000, 0xFF000000000000, 0xFF00000000000000
];
pub const EDGES: u64 = FILES[0] | FILES[7] | RANKS[0] | RANKS[7];
/// Mutable state board representing a legal chess position
#[derive(Debug)]
pub struct Board {
    pub pieces: [u64 ; 12],
    pub white_occupation: u64,
    pub black_occupation: u64,
    pub white_attacks: u64, //includes own squares (protected)
    pub black_attacks: u64,
    pub ep_square: Option<u32>, 
    pub turn: u32,
    pub nof_checkers: u32,
    pub check_block_sqrs: u64,
    pub white_pinned: u64, //bitboard of white's pinned pieces
    pub black_pinned: u64,
    //if white_pinned[sqr], then white_pinned_restrictions[sqr] contains squares that keep the pin (don't expose king)
    pub white_pinned_restrictions: [u64 ; 64],
    pub black_pinned_restrictions: [u64 ; 64],
    pub meta_attacks: u64, //when checked by scanner, this contains the squares behind the king
    pub major_minor_count: u32,
    ws: u32, //white short castling right distance
    wl: u32, //semaphore-like usage or "castling distance"
    bs: u32, //if 0, then has right else, num tells how many moves ago you had the right
    bl: u32
}

impl Board {

    ///Returns bb with both black and white occupations toggled.
    pub fn total_occupation(&self) -> u64 {
        return self.white_occupation | self.black_occupation;
    }

    pub fn is_occupied(&self, sqr: u32) -> bool {
        return self.is_white_occupied(sqr) || self.is_black_occupied(sqr);
    }

    pub fn is_occupied_by(&self, sqr: u32, by: u32) -> bool {
        if by == WHITE {
            return self.is_white_occupied(sqr);
        } else {
            return self.is_black_occupied(sqr);
        }
    }

    pub fn is_white_occupied(&self, sqr: u32) -> bool {
        return (self.white_occupation & (1 << sqr)) != 0;
    }

    pub fn is_black_occupied(&self, sqr: u32) -> bool {
        return (self.black_occupation & (1 << sqr)) != 0;
    }

    /// Gets piece of **owner** color at **sqr**. <br/>
    /// Expects that contains piece, panics if doesn't. <br/>
    /// Should only be used after checking occupany with is_{color}_occupied -method.
    pub fn get_piece_type_at(&self, sqr: u32, owner: u32) -> u32 {
        return self.lift_piece_type_at(sqr, owner).expect("sqr wasn't occupied although expected");
    }

    /// Lifts piece of **owner** color at **sqr**. <br/>
    /// Some(piece) if exists, None if no piece
    pub fn lift_piece_type_at(&self, sqr: u32, owner: u32) -> Option<u32> {
        //iterate through all piece bitboards of owner until found
        let mut p: u32;
        let e: u32;
        if owner == WHITE { p = 0; e = 6; } else { p = 6; e = 12; }
        while p < e {
            if self.pieces[p as usize] & (1 << sqr) != 0 { //found
                break;
            }
            p += 1
        }
        if p == e {
            return None;
        } else {
            return Some(p);
        }
    }
    ///Gets the idx of king's square for **side**.
    pub fn get_king_sqr_idx(&self, side: u32) -> u32 {
        if side == WHITE {
            return self.pieces[5].trailing_zeros();
        } else {
            return self.pieces[11].trailing_zeros();
        }
    }

    ///called when making a move
    pub fn update_castling_rights_make(&mut self, from: u32, to: u32, is_white_turn: bool, moved_piece: u32) {
        let short_dist: &mut u32;
        let short_corner_idx: u32;
        let long_dist: &mut u32;
        let long_corner_idx: u32;
        let king_piece_idx: u32;
        let non_mover_s_dist: &mut u32;
        let non_mover_l_dist: &mut u32;
        let non_mover_s_corner_idx: u32;
        let non_mover_l_corner_idx: u32;
        if is_white_turn { 
            short_dist = &mut self.ws; long_dist = &mut self.wl; short_corner_idx = 7; long_corner_idx = 0; king_piece_idx = W_KING; non_mover_s_dist = &mut self.bs; non_mover_l_dist = &mut self.bl;
            non_mover_s_corner_idx = 63; non_mover_l_corner_idx = 56;
        } else {
            short_dist = &mut self.bs; long_dist = &mut self.bl; short_corner_idx = 63; long_corner_idx = 56; king_piece_idx = B_KING; non_mover_s_dist = &mut self.ws; non_mover_l_dist = &mut self.wl;
            non_mover_s_corner_idx = 7; non_mover_l_corner_idx = 0;
        }
        let short_right: bool = *short_dist == 0;
        let long_right: bool = *long_dist == 0;
        //update for mover
        if moved_piece == king_piece_idx {
            *short_dist += 1;
            *long_dist += 1;
        } else { //check individual rights
            if !short_right || from == short_corner_idx || to == short_corner_idx { //check if from or to corresponding corner or already lost short right
                *short_dist += 1;
            }
            if !long_right || from == long_corner_idx || to == long_corner_idx {
                *long_dist += 1;
            }
        }
        //update for non-mover
        let non_mover_short_right: bool = *non_mover_s_dist == 0;
        let non_mover_long_right: bool = *non_mover_l_dist == 0;
        if !non_mover_short_right || to == non_mover_s_corner_idx {
            *non_mover_s_dist += 1;
        }
        if !non_mover_long_right || to == non_mover_l_corner_idx {
            *non_mover_l_dist += 1;
        }
        return;
    }

    ///Called when unmaking a move
    pub fn update_castling_rights_unmake(&mut self) {
        if self.ws != 0 {self.ws -= 1;}
        if self.wl != 0 {self.wl -= 1;}
        if self.bs != 0 {self.bs -= 1;}
        if self.bl != 0 {self.bl -= 1;}
        return;
    }

    pub fn default_board(move_gen: &MoveGen) -> Self {
        let pieces: [u64; 12] = [
            65280, 66, 36, 129, 8, 16, 71776119061217280, 4755801206503243776, 2594073385365405696, 9295429630892703744, 576460752303423488, 1152921504606846976
        ];
        let white_occupation: u64 = 0xFFFF;
        let black_occupation: u64 = 0xFFFF000000000000;
        let ep_square: Option<u32> = None;
        let (ws, wl, bs, bl) = (0, 0, 0, 0);
        let turn: u32 = WHITE;
        let major_minor_count: u32 = 14;
        return Board::board_with(pieces, white_occupation, black_occupation, turn, ws, wl, bs, bl, ep_square, major_minor_count, move_gen)
    }

    ///Returns filled in valid state board after taking minimal positional information.
    pub fn board_with(
        pieces: [u64; 12], white_occupation: u64, black_occupation: u64, turn: u32, 
        ws: u32, wl: u32, bs: u32, bl: u32, ep_square: Option<u32>, major_minor_count: u32, move_gen: &MoveGen
    ) -> Self {
        let mut res: Board = Self {
            pieces, white_occupation, black_occupation, white_attacks: 0, black_attacks: 0, ep_square, ws, wl, bs, bl, turn, white_pinned: 0, black_pinned: 0, white_pinned_restrictions: [0u64; 64], black_pinned_restrictions: [0u64; 64], nof_checkers: 0, check_block_sqrs: 0, meta_attacks: 0, major_minor_count
        }; //set computable to some defaults and compute now to get correct vals
        let non_mover_attacks: u64 = move_gen.compute_attacked(&mut res, opposite_turn(turn));
        if turn == WHITE {res.black_attacks = non_mover_attacks} else {res.white_attacks = non_mover_attacks}
        move_gen.compute_pinned(&mut res, turn);
        //now in valid state
        return res;
    }

    ///white has short castle right
    pub fn ws(&self) -> bool {
        return self.ws == 0;
    }
    ///white has long castle right
    pub fn wl(&self) -> bool {
        return self.wl == 0;
    }
    ///black has short castle right
    pub fn bs(&self) -> bool {
        return self.bs == 0;
    }
    ///black has long castle right
    pub fn bl(&self) -> bool {
        return self.bl == 0;
    }

}


impl Clone for Board {
    fn clone(&self) -> Self {
        return Self {
            pieces: self.pieces.clone(),
            white_occupation: self.white_occupation.clone(),
            black_occupation: self.black_occupation.clone(),
            white_attacks: self.white_attacks.clone(),
            black_attacks: self.black_attacks.clone(),
            ep_square: self.ep_square.clone(),
            turn: self.turn.clone(),
            nof_checkers: self.nof_checkers.clone(),
            check_block_sqrs: self.check_block_sqrs,
            white_pinned: self.white_pinned.clone(),
            black_pinned: self.black_pinned.clone(),
            white_pinned_restrictions: self.white_pinned_restrictions.clone(),
            black_pinned_restrictions: self.black_pinned_restrictions.clone(),
            meta_attacks: self.meta_attacks.clone(),
            major_minor_count: self.major_minor_count.clone(),
            ws: self.ws.clone(),
            wl: self.wl.clone(),
            bs: self.bs.clone(),
            bl: self.bl.clone()
        }
    }
}


/* impl std::fmt::Display for Board {

} */

pub const FILE_CHARS: [&str; 8] = ["a", "b", "c", "d", "e", "f", "g", "h"];

pub fn square_to_string(sqr_idx: u32) -> String {
    let mut res: String = String::new();
    res.push_str(FILE_CHARS[(sqr_idx % 8) as usize]);
    let rank: u32 = sqr_idx / 8 + 1;
    res.push_str(&rank.to_string());
    return res;
}
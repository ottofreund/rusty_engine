use crate::repr::{move_gen::MoveGen, types::{B_KING, Color, W_KING}};

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
    pub white_attacks: u64, //what squares are "seen" by white pieces, including squares with own color pieces "protected"
    pub black_attacks: u64,
    pub ep_square: Option<u32>, //if either moves pawn 2 up, resulting square comes here
    pub ws: bool, //white short castling right
    pub wl: bool,
    pub bs: bool,
    pub bl: bool,
    pub turn: Color,
    pub mover_in_check: bool,
    pub white_pinned: u64, //bitboard of white's pinned pieces
    pub black_pinned: u64,
    //IF white_pinned[sqr], then white_pinned_restrictions[sqr] contains target squares that don't break the pin i.e. stay on the pin ray (diagonal or cardinal). else is garbage and shouldn't be looked at. This 
    pub white_pinned_restrictions: [u64 ; 64],
    pub black_pinned_restrictions: [u64 ; 64]
}

impl Board {

    ///Returns bb with both black and white occupations toggled.
    pub fn total_occupation(&self) -> u64 {
        return self.white_occupation | self.black_occupation;
    }

    pub fn is_occupied(&self, sqr: u32) -> bool {
        return self.is_white_occupied(sqr) || self.is_black_occupied(sqr);
    }

    pub fn is_occupied_by(&self, sqr: u32, by: Color) -> bool {
        match by {
            Color::White => return self.is_white_occupied(sqr),
            Color::Black => return self.is_black_occupied(sqr)
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
    pub fn get_piece_type_at(&self, sqr: u32, owner: Color) -> u32 {
        return self.lift_piece_type_at(sqr, owner).unwrap();
    }

    /// Lifts piece of **owner** color at **sqr**. <br/>
    /// Some(piece) if exists, None if no piece
    pub fn lift_piece_type_at(&self, sqr: u32, owner: Color) -> Option<u32> {
        //iterate through all piece bitboards of owner until found
        let mut p: u32;
        let e: u32;
        if owner.is_white() { p = 0; e = 6; } else { p = 6; e = 12; }
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
    pub fn get_king_sqr_idx(&self, side: Color) -> u32 {
        if side.is_white() {
            return self.pieces[5].trailing_zeros();
        } else {
            return self.pieces[11].trailing_zeros();
        }
    }

    ///Called every time move is made/unmade
    pub fn update_mover_in_check(&mut self) {
        if self.turn.is_white() {
            self.mover_in_check = self.black_attacks & self.pieces[5] > 0;
        } else {
            self.mover_in_check = self.white_attacks & self.pieces[11] > 0;
        }
    }

    ///called when making/unmaking move
    pub fn update_castling_rights(&mut self, from: u32, to: u32, is_white_turn: bool, moved_piece: u32) {
        let short_right: &mut bool;
        let short_corner_idx: u32;
        let long_right: &mut bool;
        let long_corner_idx: u32;
        let king_piece_idx: u32;
        if is_white_turn { 
            short_right = &mut self.ws; long_right = &mut self.wl; short_corner_idx = 7; long_corner_idx = 0; king_piece_idx = W_KING;
        } else {
            short_right = &mut self.bs; long_right = &mut self.bl; short_corner_idx = 63; long_corner_idx = 56; king_piece_idx = B_KING;
        }
        if moved_piece == king_piece_idx {
            *short_right = false;
            *long_right = false;
            return;
        }
        //check if from or to corresponding corner
        if *short_right && (from == short_corner_idx || to == short_corner_idx) {
            *short_right = false;
        }
        if *long_right && (from == long_corner_idx || to == long_corner_idx) {
            *long_right = false;
        }
        return;
    }


    pub fn default_board(move_gen: &MoveGen) -> Self {
        let pieces: [u64; 12] = [
            65280, 66, 36, 129, 8, 16, 71776119061217280, 4755801206503243776, 2594073385365405696, 9295429630892703744, 576460752303423488, 1152921504606846976
        ];
        let white_occupation: u64 = 0xFFFF;
        let black_occupation: u64 = 0xFFFF000000000000;
        let ep_square: Option<u32> = None;
        let (ws, wl, bs, bl) = (true, true, true, true);
        let turn: Color = Color::White;
        return Board::board_with(pieces, white_occupation, black_occupation, turn, ws, wl, bs, bl, ep_square, move_gen)
    }

    pub fn board_with(
        pieces: [u64; 12], white_occupation: u64, black_occupation: u64, turn: Color, 
        ws: bool, wl: bool, bs: bool, bl: bool, ep_square: Option<u32>, move_gen: &MoveGen
    ) -> Self {
        let mut res: Board = Self {
            pieces, white_occupation, black_occupation, white_attacks: 0, black_attacks: 0, ep_square, ws, wl, bs, bl, turn, mover_in_check: false, white_pinned: 0, black_pinned: 0, white_pinned_restrictions: [0u64; 64], black_pinned_restrictions: [0u64; 64]
        }; //set computable to some defaults and compute now
        move_gen.get_all_legal(&mut res, turn.opposite());
        move_gen.get_all_legal(&mut res, turn);
        move_gen.compute_pinned(&mut res, turn);
        res.update_mover_in_check();
        //now in valid state
        return res;
    }

}

/* impl std::fmt::Display for Board {

} */

pub const file_chars: [&str; 8] = ["a", "b", "c", "d", "e", "f", "g", "h"];

pub fn square_to_string(sqr_idx: u32) -> String {
    let mut res: String = String::new();
    res.push_str(file_chars[(sqr_idx % 8) as usize]);
    let rank: u32 = sqr_idx / 8 + 1;
    res.push_str(&rank.to_string());
    return res;
}
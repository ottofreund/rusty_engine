use crate::repr::types::Color;

pub const A_FILE: u64 = 72340172838076673;
pub const H_FILE: u64 = 9259542123273814144;
pub const RANK_2: u64 = 65280;
pub const RANK_7: u64 = 71776119061217280;
pub const RANK_1: u64 = 0xFF;
pub const RANK_8: u64 = 0xFF00000000000000;
pub const EDGES: u64 = A_FILE | H_FILE | RANK_1 | RANK_8;
/// Mutable state board representing a legal chess position
#[derive(Debug)]
pub struct Board {
    pub pieces: [u64;12],
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
    pub mover_in_check: bool
}

impl Board {

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

    /// Gets piece of **owner** color at **sqr**.
    /// Expects that contains piece, panics if doesn't.
    /// Should only be used after checking occupany with is_{color}_occupied -method.
    pub fn get_piece_type_at(&self, sqr: u32, owner: Color) -> u32 {
        return self.lift_piece_type_at(sqr, owner).unwrap();
    }

    /// Lifts piece of **owner** color at **sqr**.
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

    pub fn default_board() -> Self {
        let pieces: [u64; 12] = [
            65280, 66, 36, 129, 8, 4, 71776119061217280, 4755801206503243776, 2594073385365405696, 9295429630892703744, 576460752303423488, 1152921504606846976
        ];
        let white_occupation: u64 = 0xFFFF;
        let black_occupation: u64 = 0xFFFF000000000000;
        let white_attacks: u64 = 0xFFFFFF;
        let black_attacks: u64 = 0xFFFFFF0000000000;
        let ep_square: Option<u32> = None;
        let (ws, wl, bs, bl) = (true, true, true, true);
        let turn: Color = Color::White;
        let mover_in_check: bool = false;
        return Self {
            pieces, white_occupation, black_occupation, white_attacks, black_attacks, ep_square, ws, wl, bs, bl, turn, mover_in_check
        }
    }

}

/* impl std::fmt::Display for Board {

} */

const file_chars: [&str; 8] = ["a", "b", "c", "d", "e", "f", "g", "h"];

pub fn square_to_string(sqr_idx: u32) -> String {
    let mut res: String = String::new();
    res.push_str(file_chars[(sqr_idx % 8) as usize]);
    let rank: u32 = sqr_idx / 8 + 1;
    res.push_str(&rank.to_string());
    return res;
}
use crate::repr::types::Color;

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

    pub fn make_move(&self, mov: u32) {
        return;
    }

    pub fn unmake_move(&self, mov: u32) {
        return;
    }
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
        let white_pinned: u64 = 0;
        let black_pinned: u64 = 0;
        let white_pinned_restrictions: [u64 ; 64] = [0 ; 64];
        let black_pinned_restrictions: [u64 ; 64] = [0 ; 64];
        return Self {
            pieces, white_occupation, black_occupation, white_attacks, black_attacks, ep_square, ws, wl, bs, bl, turn, mover_in_check, white_pinned, black_pinned, white_pinned_restrictions, black_pinned_restrictions
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

macro_rules! fill_rank_val {
    ($val: expr, $r: expr, $res: expr) => {
        let mut i: usize = 0;
        while i < 8 {
            $res[($r as usize) * 8 + i] = $val;
            i += 1;
        }
    }
}

macro_rules! fill_file_val {
    ($val: expr, $file: expr, $res: expr) => {
        let mut i: usize = 0;
        while i < 8 {
            $res[($file as usize) % 8 + i * 8] = $val;
            i += 1;
        }
    }
}

const fn ranks(higher: bool) -> [u64 ; 64] {
    let mut res: [u64 ; 64] = [0 ; 64];
    let mut cur: u64 = 0;
    let mut r: i32 = 7;
    //if higher {r = 7} else {r = 0};
    while r >= 0 {
        let fill_rank: i32;
        if higher {fill_rank = r} else {fill_rank = 7 - r};
        fill_rank_val!(cur, fill_rank, res);
        cur |= RANKS[fill_rank as usize]; //next one has current rank
        //if higher {r -= 1} else {r += 1};
        r -= 1;
    }
    return res;
}

const fn files(higher: bool) -> [u64 ; 64] {
    let mut res: [u64 ; 64] = [0 ; 64];
    let mut cur: u64 = 0;
    let mut f: i32 = 7;
    while f >= 0 {
        let fill_file: i32;
        if higher {fill_file = f} else {fill_file = 7 - f};
        fill_file_val!(cur, fill_file, res);
        cur |= FILES[fill_file as usize]; //next one has current rank
        //if higher {r -= 1} else {r += 1};
        f -= 1;
    }
    return res;
}
//Get bitboards with all higher/lower ranks/files than that of square_idx toggled. Used in pin computations
pub const HIGHER_RANKS: [u64 ; 64] = ranks(true);

pub const LOWER_RANKS: [u64 ; 64] = ranks(false);

pub const HIGHER_FILES: [u64 ; 64] = files(true);

pub const LOWER_FILES: [u64 ; 64] = files(false);
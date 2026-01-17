use crate::repr::{board::{Board, square_to_string}, types::*};
//moves are represented with 32 bit integers
//0-5: source square
//6-11: target square
//12: is eating move?
//13-16: free
//17: castle short?
//18: castle long?
//21: is promotion?
//22-25: promotion piece
//26-29: moved piece
//30: en passant?
//31: mover is white?
//this file contains utility methods for using them

//castling moves:
pub const WHITE_SHORT: u32 = 2147615108; //1 0000000 0000 0 0 1 0000 0 000110 000100
pub const WHITE_LONG: u32 = 2147745924; //1 0000000 0000 0 1 0 0000 0 000010 000100
pub const BLACK_SHORT: u32 = 135100; //0 0000000 0000 0 0 1 0000 0 111110 111100
pub const BLACK_LONG: u32 = 265916; //0 0000000 0000 0 1 0 0000 0 111010 111100
//corner square indices:
const WHITE_SHORT_CORNER: u32 = 7;
const WHITE_LONG_CORNER: u32 = 0;
const BLACK_SHORT_CORNER: u32 = 63;
const BLACK_LONG_CORNER: u32 = 56;

///Encode move to u32
pub fn create(from: u32, to: u32, is_take: bool, mover: Color, moved_piece: u32) -> u32 {
    let mut res: u32 = from;
    res = res | (to << 6);
    res = res | (moved_piece << 26);
    if is_take {
        res = res | 4096; // toggle 12 bit, signal is eating move
    }
    if mover.is_white() {
        res = res | 2147483648; //toggle most significant bit
    }
    return res;
}
///Castling move creator
pub fn create_castling(mover: Color, is_short: bool) -> u32 {
    if mover.is_white() {
        if is_short {
            return WHITE_SHORT;
        } else {
            return WHITE_LONG;
        }
    } else {
        if is_short {
            return BLACK_SHORT;
        } else {
            return BLACK_LONG;
        }
    }
}

/// Promotion move creator
pub fn create_promotion(from: u32, to: u32, is_take: bool, promotion_piece: u32, mover: Color, moved_piece: u32) -> u32 {
    return (create(from, to, is_take, mover, moved_piece) | 524288) | (promotion_piece << 20);
}
///Add all promotions for this pawn to a mutably borrowed move vector **vec**. Doesn't validate input, assumes correct usage
pub fn add_all_promotions(from: u32, to: u32, is_take: bool, mover: Color, moves: &mut Vec<u32>) {
    let mut p: u32;
    let e: u32;
    let moved_piece: u32;
    //start and end indices of piece based off color. Also moved piece
    if mover.is_white() { p = 1; e = 6; moved_piece = 0; } else { p = 7; e = 12; moved_piece = 6; }; 
    while p < e {
        moves.push(create_promotion(from, to, is_take, p, mover, moved_piece));
        p += 1
    }
}

//decoding methods
/// Get square moved from / init square
pub fn get_init(mov: u32) -> u32  {
    return mov & 0x3F;
}

///Get square moved to / target square 
pub fn get_target(mov: u32) -> u32  {
    return (mov & 0xFC0) >> 6;
}

///Get piece type idx
pub fn get_moved_piece(mov: u32) -> u32 {
    return (mov >> 26) & 0xF;
}

pub fn is_white_move(mov: u32) -> bool {
    return (mov & 2147483648) > 0; //msb toggled?
}

pub fn is_short_castle(mov: u32) -> bool {
    return (mov & 131072) > 0;
}

pub fn is_long_castle(mov: u32) -> bool {
    return (mov & 262144) > 0;
}

pub fn is_castle(mov: u32) -> bool {
    return is_short_castle(mov) || is_long_castle(mov);
}

pub fn is_eating(mov: u32) -> bool {
    return (mov & 4096) > 0;
}

pub fn is_promotion(mov: u32) -> bool {
    return (mov & 2097152) > 0;
}

pub fn is_en_passant(mov: u32) -> bool {
    return (mov & 1073741824) > 0;
}
//Castling rights updator's are not called for a side after both of their rights have been lost.
///Updates white's castling rights directly to board object in accordance with played move **mov**.
pub fn update_white_castling(mov: u32, board: &mut Board) {
    let init: u32 = get_init(mov);
    let target: u32 = get_target(mov);
    let moved_piece: u32 = get_moved_piece(mov);
    if moved_piece == W_KING { //remove all white castling rights
        board.ws = false;
        board.wl = false;
    } else if init == WHITE_SHORT_CORNER || target == WHITE_SHORT_CORNER { //short corner rook has moved or is eaten
        board.ws = false;
    } else if init == WHITE_LONG_CORNER || target == WHITE_LONG_CORNER {
        board.wl = false;
    }
    
}
///Updates black's castling rights directly to board object in accordance with played move **mov**.
pub fn update_black_castling(mov: u32, board: &mut Board) {
    let init: u32 = get_init(mov);
    let target: u32 = get_target(mov);
    let moved_piece: u32 = get_moved_piece(mov);
    if moved_piece == B_KING { //remove all white castling rights
        board.bs = false;
        board.bl = false;
    } else if init == BLACK_SHORT_CORNER || target == BLACK_SHORT_CORNER { //short corner rook has moved or is eaten
        board.bs = false;
    } else if init == BLACK_LONG_CORNER || target == BLACK_LONG_CORNER {
        board.bl = false;
    }
}

const piece_chars: [&str; 12] = ["P", "N", "B", "R", "Q", "K", "p", "n", "b", "r", "q", "k"];

pub fn to_string(mov: u32) -> String {
    let mut res = String::new();
    let piece: usize = get_moved_piece(mov) as usize;
    res.push_str(piece_chars[piece]);
    res.push('(');
    res.push_str(&square_to_string(get_init(mov)));
    res.push(')');
    if is_eating(mov) {
        res.push_str(" x ");
    } else {
        res.push_str(" -> ");
    }
    res.push_str(&square_to_string(get_target(mov)));
    return res;
}
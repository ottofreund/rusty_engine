use crate::repr::{board::square_to_string, types::*};
//moves are represented with 32 bit integers
//0-5: source square
//6-11: target square
//12: is eating move?
//13: is pawn double push?
//14-17: eaten piece
//18: free
//21: is promotion?
//22-25: promotion piece
//26-29: moved piece
//30: en passant?
//31: mover is white?
//this file contains utility methods for using them

pub const NULL_MOVE: u32 = u32::MAX;
//castling moves:
pub const WHITE_SHORT: u32 = 2483159428; //1 0010100 0000 0 0 1 0000 0 000110 000100
pub const WHITE_LONG: u32 = 2483290244; //1 0010100 0000 0 1 0 0000 0 000010 000100
pub const BLACK_SHORT: u32 = 738332604; //0 0101100 0000 0 0 1 0000 0 111110 111100
pub const BLACK_LONG: u32 = 738463420; //0 0101100 0000 0 1 0 0000 0 111010 111100

///Encode move to u32 <br>
///taken piece idx is found and added after checking pseudolegal is legal to save compute
pub fn create(from: u32, to: u32, is_take: bool, mover: u32, moved_piece: u32) -> u32 {
    let mut res: u32 = from;
    res = res | (to << 6);
    res = res | (moved_piece << 26);
    if is_take {
        res = res | 4096; // toggle 12 bit, signal is eating move
    }
    if mover == WHITE {
        res = res | 2147483648; //toggle most significant bit
    }
    return res;
}
///Castling move creator
pub fn create_castling(mover: u32, is_short: bool) -> u32 {
    if mover == WHITE {
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

///Pawn double push move
pub fn create_double_push(from: u32, to: u32, mover: u32, moved_piece: u32) -> u32 {
    return create(from, to, false, mover, moved_piece) | 8192; // | 2^13
}

///**to** is the ep_square the pawn ends up on, eaten pawn must be cleared in make_move with some extra logic
pub fn create_en_passant(from: u32, to: u32, mover: u32, moved_piece: u32) -> u32 {
    return create(from, to, true, mover, moved_piece) | 1073741824; // | 2^30
}

/// Promotion move creator
pub fn create_promotion(
    from: u32,
    to: u32,
    is_take: bool,
    promotion_piece: u32,
    mover: u32,
    moved_piece: u32,
) -> u32 {
    return (create(from, to, is_take, mover, moved_piece) | 2097152) | (promotion_piece << 22);
    //2097152 == 2^21
}
///Add all promotions for this pawn to a mutably borrowed move vector **vec**. Doesn't validate input, assumes correct usage
pub fn add_all_promotions(from: u32, to: u32, is_take: bool, mover: u32, moves: &mut Vec<u32>) {
    let mut p: u32;
    let e: u32;
    let moved_piece: u32;
    //start and end indices of piece based off color. Also moved piece
    if mover == WHITE {
        p = W_KNIGHT;
        e = W_KING;
        moved_piece = W_PAWN;
    } else {
        p = B_KNIGHT;
        e = B_KING;
        moved_piece = B_PAWN;
    };
    while p < e {
        moves.push(create_promotion(from, to, is_take, p, mover, moved_piece));
        p += 1
    }
}

///Add only queen and knight promotions to a mutably borrowed move vector **vec**. Doesn't validate input, assumes correct usage
pub fn add_search_promotions(from: u32, to: u32, is_take: bool, mover: u32, moves: &mut Vec<u32>) {
    let moved_pawn_idx: u32;
    let queen_piece_idx: u32;
    let knight_piece_idx: u32;
    if mover == WHITE {
        moved_pawn_idx = W_PAWN;
        queen_piece_idx = W_QUEEN;
        knight_piece_idx = W_KNIGHT;
    } else {
        moved_pawn_idx = B_PAWN;
        queen_piece_idx = B_QUEEN;
        knight_piece_idx = B_KNIGHT;
    }
    moves.push(create_promotion(
        from,
        to,
        is_take,
        queen_piece_idx,
        mover,
        moved_pawn_idx,
    ));
    moves.push(create_promotion(
        from,
        to,
        is_take,
        knight_piece_idx,
        mover,
        moved_pawn_idx,
    ));
    return;
}

//decoding methods
/// Get square moved from / init square
#[inline]
pub fn get_init(mov: u32) -> u32 {
    return mov & 0x3F;
}

///Get square moved to / target square
#[inline]
pub fn get_target(mov: u32) -> u32 {
    return (mov & 0xFC0) >> 6;
}

///Get piece type idx
#[inline]
pub fn get_moved_piece(mov: u32) -> u32 {
    return (mov >> 26) & 0xF;
}

#[inline]
pub fn get_promoted_piece(mov: u32) -> u32 {
    return (mov >> 22) & 0xF;
}

#[inline]
pub fn is_white_move(mov: u32) -> bool {
    return (mov & 2147483648) > 0; //msb toggled?
}

#[inline]
pub fn is_short_castle(mov: u32) -> bool {
    return mov == WHITE_SHORT || mov == BLACK_SHORT;
}

#[inline]
pub fn is_long_castle(mov: u32) -> bool {
    return mov == WHITE_LONG || mov == BLACK_LONG;
}

#[inline]
pub fn is_castle(mov: u32) -> bool {
    return is_short_castle(mov) || is_long_castle(mov);
}

#[inline]
pub fn is_eating(mov: u32) -> bool {
    return (mov & 4096) > 0;
}

#[inline]
pub fn is_promotion(mov: u32) -> bool {
    return (mov & 2097152) > 0;
}

#[inline]
pub fn is_en_passant(mov: u32) -> bool {
    return (mov & 1073741824) > 0; //2^30
}

#[inline]
pub fn is_double_push(mov: u32) -> bool {
    return (mov & 8192) > 0;
}

pub fn eaten_piece(mov: u32) -> Option<u32> {
    if is_eating(mov) {
        return Some((mov >> 14) & 0xF);
    } else {
        return None;
    }
}

pub fn with_eaten_piece(mov: u32, eaten: u32) -> u32 {
    return mov | (eaten << 14);
}

pub fn breaks_fifty_move(mov: u32) -> bool {
    return is_eating(mov) || (get_moved_piece(mov) % 6) == W_PAWN;
}

pub fn is_unrepeatable(mov: u32) -> bool {
    return is_eating(mov) || (get_moved_piece(mov) % 6) == W_PAWN || is_castle(mov);
}

pub fn breaks_fifty_counter(mov: u32) -> bool {
    return is_eating(mov) || (get_moved_piece(mov) % 6) == W_PAWN;
}

const PIECE_CHARS: [&str; 12] = ["P", "N", "B", "R", "Q", "K", "p", "n", "b", "r", "q", "k"];

pub fn to_string(mov: u32) -> String {
    if mov == NULL_MOVE {
        return "NULL_MOVE".to_string();
    }
    let mut res = String::new();
    let piece: usize = get_moved_piece(mov) as usize;
    res.push_str(PIECE_CHARS[piece]);
    res.push('(');
    res.push_str(&square_to_string(get_init(mov)));
    res.push(')');
    if is_eating(mov) {
        res.push_str(" x ");
    } else {
        res.push_str(" -> ");
    }
    res.push_str(&square_to_string(get_target(mov)));
    if is_promotion(mov) {
        res.push_str(" -- promoting to ");
        res.push_str(PIECE_CHARS[get_promoted_piece(mov) as usize]);
    }
    return res;
}

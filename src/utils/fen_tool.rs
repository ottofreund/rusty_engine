use crate::repr::bitboard;
use crate::repr::board::Board;
use crate::repr::move_gen::MoveGen;
use crate::repr::types::{B_KING, B_PAWN, B_ROOK, BLACK, W_KING, W_PAWN, W_ROOK, WHITE};
use crate::utils::zobrist::Zobrist;

const VALID_PIECE_CHARS: [char; 12] = ['P', 'N', 'B', 'R', 'Q', 'K', 'p', 'n', 'b', 'r', 'q', 'k'];
const VALID_MOVER_CHARS: [char; 2] = ['w', 'b'];
const VALID_CASTLING_CHARS: [char; 4] = ['K', 'Q', 'k', 'q'];
const FILE_CHARS: [char; 8] = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
pub const MAJOR_MINOR_PIECES: [char; 8] = ['N', 'B', 'R', 'Q', 'n', 'b', 'r', 'q'];

pub fn is_valid_fen(fen: &String) -> bool {
    let mut sections = fen.split_whitespace();
    let sec_count: usize = sections.clone().count();
    if sec_count < 4 || sec_count > 6 {
        return false;
    }
    let pieces: &str = sections
        .next()
        .expect("Was long enough but iterator ended.");
    let rows = pieces.split('/');
    if rows.clone().count() != 8 {
        return false;
    }
    let mut white_king: bool = false;
    let mut black_king: bool = false;
    for row in rows.clone() {
        if !is_legal_piece_row(row) {
            return false;
        }
        if row.contains('K') {
            if white_king {
                return false;
            }
            white_king = true;
        }
        if row.contains('k') {
            if black_king {
                return false;
            }
            black_king = true;
        }
    }
    //both sides must have king
    if !white_king || !black_king {
        return false;
    }
    //second section is mover
    let mover: &str = sections
        .next()
        .expect("Was long enough but iterator ended.");
    if mover.len() != 1 || !VALID_MOVER_CHARS.contains(&mover.chars().collect::<Vec<char>>()[0]) {
        return false;
    }
    //third section castling rights
    let castling_rights: &str = sections
        .next()
        .expect("Was long enough but iterator ended.");
    if castling_rights != "-" {
        if castling_rights.len() > 4 {
            return false;
        }
        let mut lowercase_seen: bool = false;
        for c in castling_rights.chars() {
            if (c.is_uppercase() && lowercase_seen) || !VALID_CASTLING_CHARS.contains(&c) {
                return false;
            }
            if c.is_lowercase() {
                lowercase_seen = true;
            }
        }
    }
    //last section en passant
    let ep: &str = sections
        .next()
        .expect("Was long enough but iterator ended.");
    if ep != "-" {
        if ep.len() != 2 {
            return false;
        }
        let chars: Vec<char> = ep.chars().collect::<Vec<char>>();
        if !FILE_CHARS.contains(&chars[0]) || !chars[1].is_digit(10) {
            return false;
        }
        let rank: u32 = chars[1].to_digit(10).expect("Checked is_digit, wasn't");
        if rank < 1 || rank > 8 {
            return false;
        }
    }
    if let Some(half_move_clock) = sections.next() {
        if half_move_clock.parse::<u32>().is_err() {
            return false;
        }
    }
    if let Some(full_move_number) = sections.next() {
        if full_move_number.parse::<u32>().is_err() {
            return false;
        }
    }
    return true;
}

fn is_legal_piece_row(row: &str) -> bool {
    let mut piece_count: u32 = 0;

    for c in row.chars() {
        if c.is_digit(10) {
            //empty spaces
            piece_count += c.to_digit(10).expect("Checked is_digit but wasn't");
        } else if VALID_PIECE_CHARS.contains(&c) {
            piece_count += 1;
        } else {
            return false;
        }
    }
    if piece_count != 8 {
        return false;
    }
    return true;
}


fn castling_pieces_on_right_squares(castling_str: &str, board: &Board) -> bool {
    if castling_str.contains('K') && 
        (  !bitboard::contains_square(board.pieces[W_KING as usize], 4) 
        || !bitboard::contains_square(board.pieces[W_ROOK as usize], 7)
    ) {
        return false;
    }
    if castling_str.contains('Q') && 
        (  !bitboard::contains_square(board.pieces[W_KING as usize], 4) 
        || !bitboard::contains_square(board.pieces[W_ROOK as usize], 0)
    ) {
        return false;
    }
    if castling_str.contains('k') && 
        (  !bitboard::contains_square(board.pieces[B_KING as usize], 60) 
        || !bitboard::contains_square(board.pieces[B_ROOK as usize], 63)
    ) {
        return false;
    }
    if castling_str.contains('q') && 
        (  !bitboard::contains_square(board.pieces[B_KING as usize], 60) 
        || !bitboard::contains_square(board.pieces[B_ROOK as usize], 56)
    ) {
        return false;
    }
    return true;
}

fn ep_square_consistent_with_pieces(ep_square: Option<u32>, board: &Board) -> bool {
    if let Some(square) = ep_square {
        let rank = square / 8;
        if rank == 2 {
            //white pawn must be on rank 4
            if !bitboard::contains_square(board.pieces[W_PAWN as usize], square + 8) {
                return false;
            }
        } else if rank == 5 {
            //black pawn must be on rank 5
            if !bitboard::contains_square(board.pieces[B_PAWN as usize], square - 8) {
                return false;
            }
        } else {
            return false;
        }
    }
    return true;
}

///returns major minor count
fn parse_pieces(
    piece_str: &str,
    pieces: &mut [u64; 12],
    white_occupation: &mut u64,
    black_occupation: &mut u64,
) -> u32 {
    let rows = piece_str.split('/');
    let mut sqr_idx: i32 = 56;
    let mut major_minor_count = 0;
    for row in rows.clone() {
        for c in row.chars() {
            if c.is_alphabetic() {
                let piece_type: usize = VALID_PIECE_CHARS
                    .iter()
                    .position(|p| *p == c)
                    .expect("Was valid fen but unknown piece type");
                //bitboard::set_square(&mut pieces[piece_type], sqr_idx as u32);
                pieces[piece_type] |= 1 << sqr_idx;
                if c.is_uppercase() {
                    //white
                    //bitboard::set_square(white_occupation, sqr_idx as u32);
                    *white_occupation |= 1 << sqr_idx;
                } else {
                    //black
                    //bitboard::set_square(black_occupation, sqr_idx as u32);
                    *black_occupation |= 1 << sqr_idx;
                }
                if MAJOR_MINOR_PIECES.contains(&c) {
                    major_minor_count += 1;
                }
                sqr_idx += 1;
            } else {
                //empty for c spaces
                let spaces: u32 = c
                    .to_digit(10)
                    .expect("Was valid fen but invalid space count");
                sqr_idx += spaces as i32;
            }
        }
        sqr_idx -= 16
    }
    return major_minor_count;
}

///Ok(board) with board being filled in valid state board, if fen valid <br>
///Else Err(FenError)
pub fn fen_to_board(
    fen: String,
    move_gen: &MoveGen,
    zobrist: &Zobrist,
) -> Result<Board, &'static str> {
    if !is_valid_fen(&fen) {
        return Err("Fen error");
    }
    let mut pieces: [u64; 12] = [0; 12];
    let mut white_occupation: u64 = 0;
    let mut black_occupation: u64 = 0;
    let mut sections = fen.split_whitespace();
    let piece_str: &str = sections
        .next()
        .expect("Was long enough but iterator ended.");
    let major_minor_count: u32 = parse_pieces(
        piece_str,
        &mut pieces,
        &mut white_occupation,
        &mut black_occupation,
    );
    let turn: u32 = match sections.next().expect("Was valid but sections ran out") {
        "w" => WHITE,
        "b" => BLACK,
        _ => panic!("Turn was not 'w' or 'b' in fen that was valid."),
    };
    let castling_string: &str = sections.next().expect("Was valid but sections ran out");
    let ws: bool = castling_string.contains('K');
    let wl: bool = castling_string.contains('Q');
    let bs: bool = castling_string.contains('k');
    let bl: bool = castling_string.contains('q');

    let ep_string: &str = sections.next().expect("Was valid but sections ran out");
    let ep_square: Option<u32>;
    if ep_string == "-" {
        ep_square = None;
    } else {
        let mut char_iter = ep_string.chars();
        let file: char = char_iter.next().expect("ep wasn't valid");
        let rank: u32 = char_iter.next().expect("ep wasn't valid") as u32 - b'1' as u32;
        ep_square = Some(file as u32 - 'a' as u32 + 8 * rank);
    }
    let half_move_clock: u32 = match sections.next() {
        Some(half_move_clock_string) => half_move_clock_string
            .parse::<u32>()
            .expect("Was valid but half move clock wasn't a u32"),
        None => 0,
    };

    let board: Board = Board::board_with(
        pieces,
        white_occupation,
        black_occupation,
        turn,
        !ws as u32,
        !wl as u32,
        !bs as u32,
        !bl as u32,
        ep_square,
        major_minor_count,
        move_gen,
        zobrist,
        half_move_clock,
    );
    if !castling_pieces_on_right_squares(castling_string, &board) {
        return Err("FEN error: castling rights not consistent with piece placement");
    }
    if !ep_square_consistent_with_pieces(ep_square, &board) {
        return Err("FEN error: en passant square not consistent with piece placement");
    }

    return Ok(board);
}

pub fn board_to_fen(board: &Board) -> String {
    let mut fen = String::with_capacity(64);

    for rank in (0usize..8).rev() {
        let mut empty_count = 0;

        for file in 0usize..8 {
            let square = rank * 8 + file;

            let piece = board
                .pieces
                .iter()
                .position(|bitboard| (*bitboard >> square) & 1 != 0)
                .map(|index| VALID_PIECE_CHARS[index]);

            match piece {
                Some(piece_char) => {
                    if empty_count != 0 {
                        fen.push(char::from_digit(empty_count, 10).unwrap());
                        empty_count = 0;
                    }
                    fen.push(piece_char);
                }
                None => empty_count += 1,
            }
        }

        if empty_count != 0 {
            fen.push(char::from_digit(empty_count, 10).unwrap());
        }

        if rank != 0 {
            fen.push('/');
        }
    }

    fen.push(' ');
    fen.push(if board.turn == WHITE { 'w' } else { 'b' });
    fen.push(' ');

    let castling_start = fen.len();

    if board.ws() {
        fen.push('K');
    }
    if board.wl() {
        fen.push('Q');
    }
    if board.bs() {
        fen.push('k');
    }
    if board.bl() {
        fen.push('q');
    }

    if fen.len() == castling_start {
        fen.push('-');
    }

    fen.push(' ');

    match board.ep_square {
        Some(square) => {
            fen.push(char::from(b'a' + (square % 8) as u8));
            fen.push(char::from(b'1' + (square / 8) as u8));
        }
        None => fen.push('-'),
    }

    fen.push(' ');
    fen.push_str(&board.half_move_clock.to_string());
    fen.push_str(" 1");

    fen
}

pub const DEFAULT_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

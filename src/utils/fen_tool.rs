
const valid_piece_chars: [char ; 12] = ['P', 'N', 'B', 'R', 'Q', 'K', 'p', 'n', 'b', 'r', 'q', 'k'];
const valid_mover_chars: [char ; 2] = ['w', 'b'];
const valid_castling_chars: [char ; 4] = ['K', 'Q', 'k', 'q'];
const file_chars: [char; 8] = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];


pub fn is_valid_fen(fen: &String) -> bool {
    let mut sections = fen.split(' ');
    let sec_count: usize = sections.clone().count();
    if sec_count < 4 || sec_count > 6 {
        return false;
    }
    let pieces: &str = sections.next().expect("Was long enough but iterator ended.");
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
        if row.contains('K') {white_king = true;}
        if row.contains('k') {black_king = true;}
    }
    //both sides must have king
    if !white_king || !black_king {
        return false;
    }
    //second section is mover
    let mover: &str = sections.next().expect("Was long enough but iterator ended.");
    if mover.len() != 1 || !valid_mover_chars.contains(&mover.chars().collect::<Vec<char>>()[0]) {
        return false;
    }
    //third section castling rights
    let castling_rights: &str = sections.next().expect("Was long enough but iterator ended.");
    if castling_rights != "-" {
        if castling_rights.len() > 4 {
            return false;
        }
        let mut lowercase_seen: bool = false;
        for c in castling_rights.chars() {
            if (c.is_uppercase() && lowercase_seen) || !valid_castling_chars.contains(&c) {
                return false;
            }
            if c.is_lowercase() {lowercase_seen = true;}
        }
    }
    //last section en passant
    let ep: &str = sections.next().expect("Was long enough but iterator ended.");
    if ep != "-" {
        if ep.len() != 2 {
            return false;
        }
        let chars: Vec<char> = ep.chars().collect::<Vec<char>>();
        if !file_chars.contains(&chars[0]) || !chars[1].is_digit(10) {
            return false;
        }
        let rank: u32 = chars[1].to_digit(10).expect("Checked is_digit, wasn't");
        if rank < 1 || rank > 8 {
            return false;
        }
    }
    return true;
}


pub fn is_legal_piece_row(row: &str) -> bool {
    let mut piece_count: u32 = 0;

    for c in row.chars() {
        if c.is_digit(10) { //empty spaces
            piece_count += c.to_digit(10).expect("Checked is_digit but wasn't");
        } else if valid_piece_chars.contains(&c) {
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


pub fn parse_pieces(piece_str: &str, pieces: &mut [u64 ; 12], white_occupation: &mut u64, black_occupation: &mut u64) {
    let rows = piece_str.split('/');
    let mut sqr_idx: i32 = 56;
    for row in rows.clone() {
        for c in row.chars() {
            if c.is_alphabetic() {
                let piece_type: usize = valid_piece_chars.iter().position(|p| *p == c).expect("Was valid fen but unknown piece type");
                //bitboard::set_square(&mut pieces[piece_type], sqr_idx as u32);
                pieces[piece_type] |= 1 << sqr_idx;
                if c.is_uppercase() { //white
                    //bitboard::set_square(white_occupation, sqr_idx as u32);
                    *white_occupation |= 1 << sqr_idx;
                } else { //black
                    //bitboard::set_square(black_occupation, sqr_idx as u32);
                    *black_occupation |= 1 << sqr_idx;
                }
                sqr_idx += 1;
            } else { //empty for c spaces
                let spaces: u32 = c.to_digit(10).expect("Was valid fen but invalid space count");
                sqr_idx += spaces as i32;
            }
        }
        sqr_idx -= 16
    }
    return;
}

pub const DEFAULT_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
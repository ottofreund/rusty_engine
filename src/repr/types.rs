pub const W_PAWN: u32 = 0;
pub const W_KNIGHT: u32 = 1;
pub const W_BISHOP: u32 = 2;
pub const W_ROOK: u32 = 3;
pub const W_QUEEN: u32 = 4;
pub const W_KING: u32 = 5;
pub const B_PAWN: u32 = 6;
pub const B_KNIGHT: u32 = 7;
pub const B_BISHOP: u32 = 8;
pub const B_ROOK: u32 = 9;
pub const B_QUEEN: u32 = 10;
pub const B_KING: u32 = 11;

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Color {
    White,
    Black
}

impl Color {
    pub fn opposite(self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White
        }
    }

    pub fn is_white(self) -> bool {
        return self == Color::White;
    }

}


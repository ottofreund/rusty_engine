#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    SquareClicked(u32),
    Reset,
}
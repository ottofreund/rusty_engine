

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    SquareClicked(u32),
    Reset,
    Unmake,
    Event(iced::Event)
}



#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    SquareClicked(u32),
    Reset,
    Unmake,
    NewFenPosPressed, //takes fen_str and side (0 white 1 black)
    NewDefaultPosPressed,
    FenContentChanged(String),
    ErrorHandled,
    InputSideWhitePressed,
    InputSideBlackPressed,
    Event(iced::Event)
}

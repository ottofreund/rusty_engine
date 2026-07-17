#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    SquareClicked(u32),
    Unmake,
    NewFenPosPressed, //takes fen_str and side (0 white 1 black)
    NewDefaultPosPressed,
    FenContentChanged(String),
    ErrorHandled,
    InputSideWhitePressed,
    InputSideBlackPressed,
    Event(iced::Event),
    SearchStart,
    PromotionSelected(&'static str),
    GameEndAcknowledged,
    ErrorAcknowledged,
}

pub use super::evaluation_history::Evaluation;

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    SquareClicked(u32),
    Unmake,
    NewFenPosPressed, //takes fen_str and side (0 white 1 black)
    NewDefaultPosPressed,
    TimeControlSelected(super::clock::TimeControl),
    CustomMinutes(String),
    CustomSeconds(String),
    CustomIncrement(String),
    FenContentChanged(String),
    ErrorHandled,
    InputSideWhitePressed,
    InputSideBlackPressed,
    Navigate(super::history::Navigation),
    EvaluationHistoryToggled(bool),
    SearchStart,
    ThinkingTick,
    SearchFinished(u64, Option<u32>, Option<Evaluation>),
    PromotionSelected(&'static str),
    GameEndAcknowledged,
    ErrorAcknowledged,
}


pub enum GameState {
    InProgress,
    Checkmate(u32), //WHITE == 0 or BLACK == 1
    Stalemate,
    DrawByRepetition,
    DrawByFiftyMoveRule,
    DrawByInsufficientMaterial,
    DrawByTimeout
}

impl GameState {
    pub fn is_draw(&self) -> bool {
        match self {
            GameState::DrawByRepetition => true,
            GameState::DrawByFiftyMoveRule => true,
            GameState::DrawByInsufficientMaterial => true,
            GameState::DrawByTimeout => true,
            _ => false
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            GameState::InProgress => "In Progress".to_string(),
            GameState::Checkmate(turn) => format!("Checkmate, {} wins", if *turn == 0 { "Black" } else { "White" }),
            GameState::Stalemate => "Stalemate".to_string(),
            GameState::DrawByRepetition => "Draw by Repetition".to_string(),
            GameState::DrawByFiftyMoveRule => "Draw by Fifty Move Rule".to_string(),
            GameState::DrawByInsufficientMaterial => "Draw by Insufficient Material".to_string(),
            GameState::DrawByTimeout => "Draw by Timeout".to_string()
        }
    }

}
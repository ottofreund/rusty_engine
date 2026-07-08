use crate::{repr::_move, utils::fen_tool::DEFAULT_FEN};

pub enum ArbiterCommand {
    UCI,
    IsReady,
    SetOption(_Option),
    UCINewGame,
    Go(GoCommand),
    PonderHit,
    Position(PositionCommand),
    Quit,
    Stop,
}

pub enum EngineCommand {
    ID(String),
    BestMove(String, Option<String>),
    Option(OptionCommand),
}

impl EngineCommand {
    pub fn default_id() -> Self {
        Self::ID("Rusty".into())
    }

    pub fn new_best_move(best_move: u32, ponder: Option<u32>) -> Self {
        let best_move_str = _move::to_string(best_move, true);
        let ponder_str = match ponder {
            Some(p) => Some(_move::to_string(p, true)),
            None => None,
        };
        return Self::BestMove(best_move_str, ponder_str);
    }
}

pub enum _Option {
    Ponder(String), //option name
}

pub enum OptionType {
    Check(String), //type name
}

pub struct GoCommand {
    pub ponder: bool,
    pub wtime: Option<u64>,
    pub btime: Option<u64>,
    pub winc: Option<u64>,
    pub binc: Option<u64>,
    pub movetime: Option<u64>,
}

impl GoCommand {
    pub fn is_valid(&self) -> bool {
        if self.wtime.is_some() && self.btime.is_some() {
            return true;
        }
        if self.movetime.is_some() {
            return true;
        }
        return false;
    }

    pub fn new_clock_tc(ponder: bool, wtime: u64, btime: u64, winc: u64, binc: u64) -> Self {
        Self {
            ponder: ponder,
            wtime: Some(wtime),
            btime: Some(btime),
            winc: Some(winc),
            binc: Some(binc),
            movetime: None,
        }
    }

    pub fn new_movetime_tc(ponder: bool, movetime: u64) -> Self {
        Self {
            ponder: ponder,
            wtime: None,
            btime: None,
            winc: None,
            binc: None,
            movetime: Some(movetime),
        }
    }
}

pub struct PositionCommand {
    pub fen: String,
    pub moves: Vec<String>,
}

impl PositionCommand {
    pub fn new(fen: Option<String>, moves: Vec<String>) -> Self {
        return Self {
            fen: fen.unwrap_or_else(|| DEFAULT_FEN.into()),
            moves,
        };
    }
}

struct OptionCommand {
    pub option: _Option,
    pub default_value: String,
}

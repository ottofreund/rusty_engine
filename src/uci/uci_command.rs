use crate::{
    repr::_move, search::searcher::MAX_SEARCH_DEPTH, utils::fen_tool::DEFAULT_FEN,
};

pub enum ArbiterCommand {
    UCI,
    Display,
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

#[derive(Debug)]
pub enum _Option {
    Ponder(String), //option name
}

pub enum OptionType {
    Check(String), //type name
}

#[derive(Clone)]
pub struct GoCommand {
    pub ponder: bool,
    pub wtime: Option<u64>,
    pub btime: Option<u64>,
    pub winc: Option<u64>,
    pub binc: Option<u64>,
    pub movetime: Option<u64>,
    pub depth: Option<usize>,
}

impl GoCommand {
    pub fn is_valid(&self) -> bool {
        let has_clock = self.wtime.is_some() && self.btime.is_some();
        let has_partial_clock = self.wtime.is_some() != self.btime.is_some();
        let mode_count =
            has_clock as u8 + self.movetime.is_some() as u8 + self.depth.is_some() as u8;

        !has_partial_clock
            && mode_count == 1
            && self
                .depth
                .map_or(true, |depth| depth <= MAX_SEARCH_DEPTH)
    }

    pub fn new_clock_tc(ponder: bool, wtime: u64, btime: u64, winc: u64, binc: u64) -> Self {
        Self {
            ponder: ponder,
            wtime: Some(wtime),
            btime: Some(btime),
            winc: Some(winc),
            binc: Some(binc),
            movetime: None,
            depth: None,
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
            depth: None,
        }
    }

    pub fn new_depth_tc(ponder: bool, depth: usize) -> Self {
        Self {
            ponder: ponder,
            wtime: None,
            btime: None,
            winc: None,
            binc: None,
            movetime: None,
            depth: Some(depth),
        }
    }
}

#[derive(Clone)]
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

    pub fn preceeds(&self, other: &PositionCommand) -> bool {
        if self.fen != other.fen {
            return false;
        }
        if self.moves.len() + 1 != other.moves.len() {
            return false;
        }
        for (i, m) in self.moves.iter().enumerate() {
            if m != &other.moves[i] {
                return false;
            }
        }
        return true;
    }

}

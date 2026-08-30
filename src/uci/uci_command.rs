use std::fmt;

use crate::{
    repr::_move,
    search::{searcher::MAX_SEARCH_DEPTH, tt::DEFAULT_TT_SIZE_MB},
    utils::fen_tool::DEFAULT_FEN,
};

pub enum ArbiterCommand {
    UCI,
    Display,
    IsReady,
    SetOption(SetOptionCommand),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetOptionCommand {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum UciOptionId {
    Hash,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum UciOptionKind {
    Spin { default: i64, min: i64, max: i64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct UciOptionSpec {
    pub id: UciOptionId,
    pub name: &'static str,
    pub kind: UciOptionKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum UciOptionValue {
    Spin(i64),
}

pub(super) const UCI_OPTIONS: &[UciOptionSpec] = &[UciOptionSpec {
    id: UciOptionId::Hash,
    name: "Hash",
    kind: UciOptionKind::Spin {
        default: DEFAULT_TT_SIZE_MB as i64,
        min: 1,
        max: 32_768,
    },
}];

impl fmt::Display for UciOptionSpec {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "option name {}", self.name)?;
        match self.kind {
            UciOptionKind::Spin { default, min, max } => write!(
                formatter,
                " type spin default {default} min {min} max {max}"
            ),
        }
    }
}

pub(super) fn resolve_uci_option(
    command: &SetOptionCommand,
) -> Option<(UciOptionId, UciOptionValue)> {
    let option = UCI_OPTIONS
        .iter()
        .find(|option| option.name.eq_ignore_ascii_case(&command.name))?;

    match option.kind {
        UciOptionKind::Spin { min, max, .. } => {
            let value = command.value.as_deref()?.parse::<i64>().ok()?;
            (min..=max)
                .contains(&value)
                .then_some((option.id, UciOptionValue::Spin(value)))
        }
    }
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

    /// Returns (does_preceeds, how_many_moves)
    pub fn preceeds(&self, other: &PositionCommand) -> (bool, usize) {
        if self.fen != other.fen {
            return (false, 0);
        }
        if self.moves.len() >= other.moves.len() {
            return (false, 0);
        }
        for (i, m) in self.moves.iter().enumerate() {
            if m != &other.moves[i] {
                return (false, 0);
            }
        }
        return (true, other.moves.len() - self.moves.len());
    }

}

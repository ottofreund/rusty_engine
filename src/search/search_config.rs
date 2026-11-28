use std::{
    sync::{Arc, OnceLock},
    time::Instant,
};

#[cfg(debug_assertions)]
const SEARCH_RESPONSE_MARGIN: u64 = 65; //ms
#[cfg(not(debug_assertions))]
const SEARCH_RESPONSE_MARGIN: u64 = DEFAULT_MOVE_OVERHEAD_MS;

pub(crate) const DEFAULT_MOVE_OVERHEAD_MS: u64 = 15;
const DEFAULT_STATIC_TIME: u64 = 5000;

///Config defining HOW we search. <br>
///Might be mutated in between searches to dynamically adjust behaviour.
pub struct SearchConfig {
    pub search_mode: SearchMode,
    pub quiescence: bool,
    pub log_diagnostics: bool,
    pub log_uci_diagnostics: bool,
    pub(crate) ponder_start: Option<Arc<OnceLock<Instant>>>,
    pub(crate) move_overhead_ms: u64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            search_mode: SearchMode::StaticTime(DEFAULT_STATIC_TIME),
            quiescence: true,
            log_diagnostics: false,
            log_uci_diagnostics: true,
            ponder_start: None,
            move_overhead_ms: DEFAULT_MOVE_OVERHEAD_MS,
        }
    }
}

impl SearchConfig {
    pub fn with_d(d: usize) -> Self {
        Self {
            search_mode: SearchMode::StaticDepth(d),
            quiescence: true,
            log_diagnostics: false,
            log_uci_diagnostics: true,
            ponder_start: None,
            move_overhead_ms: DEFAULT_MOVE_OVERHEAD_MS,
        }
    }
}

pub enum SearchMode {
    StaticDepth(usize),
    StaticTime(u64), //ms
    Clock(TimeBudget),
    Infinite,
}

impl SearchMode {
    ///t: time in ms
    pub fn static_time_with_margin(t: u64) -> Self {
        Self::StaticTime(t.saturating_sub(SEARCH_RESPONSE_MARGIN))
    }

    pub fn time_control_with_margin(
        wtime: u64,
        btime: u64,
        winc: u64,
        binc: u64,
        is_white_turn: bool,
    ) -> Self {
        let time_left = if is_white_turn { wtime } else { btime };
        let inc = if is_white_turn { winc } else { binc };
        Self::Clock(TimeBudget::new(
            time_left,
            inc,
            None,
            SEARCH_RESPONSE_MARGIN,
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeBudget {
    hard_ms: u64,
}

impl TimeBudget {
    pub fn new(remaining: u64, increment: u64, _moves_to_go: Option<u64>, overhead: u64) -> Self {
        let hard_ms = (remaining / 20)
            .saturating_add(increment)
            .min(remaining)
            .saturating_sub(overhead);
        Self { hard_ms }
    }

    pub fn hard_ms(self) -> u64 {
        self.hard_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_budget_caps_against_remaining_time_before_overhead() {
        assert_eq!(TimeBudget::new(2000, 100, None, 15).hard_ms(), 185);
        assert_eq!(TimeBudget::new(2000, 100, Some(1), 15).hard_ms(), 185);
        assert_eq!(TimeBudget::new(10, 1000, None, 15).hard_ms(), 0);
        assert_eq!(TimeBudget::new(45, u64::MAX, None, 15).hard_ms(), 30);
    }
}

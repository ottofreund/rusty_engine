use std::{
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
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
    soft_ms: u64,
    hard_ms: u64,
}

impl TimeBudget {
    pub fn new(remaining: u64, increment: u64, moves_to_go: Option<u64>, overhead: u64) -> Self {
        let available = remaining.saturating_sub(overhead);
        let horizon = moves_to_go.unwrap_or(28).clamp(1, 60);
        if horizon == 1 {
            return Self {
                soft_ms: available - available / 5,
                hard_ms: available,
            };
        }

        // The increment for this move arrives after bestmove, so only spend a
        // portion of it and cap the budget against the clock we already have.
        let max_for_move = available - available / 3;
        let soft_ms = (available / horizon)
            .saturating_add(increment - increment / 4)
            .max(1)
            .min(max_for_move);
        let hard_ms = soft_ms.saturating_mul(4).max(5).min(max_for_move);
        Self { soft_ms, hard_ms }
    }

    pub fn hard_ms(self) -> u64 {
        self.hard_ms
    }
}

pub(super) struct TimeManager {
    budget: TimeBudget,
    best_move: Option<u32>,
    best_score: i16,
    stable_iterations: u32,
}

impl TimeManager {
    pub(super) fn new(budget: TimeBudget) -> Self {
        Self {
            budget,
            best_move: None,
            best_score: 0,
            stable_iterations: 0,
        }
    }

    pub(super) fn should_start_iteration(
        &mut self,
        elapsed: Duration,
        last_iteration: Duration,
        best_move: u32,
        score: i16,
        best_move_nodes_pct: Option<u8>,
    ) -> bool {
        let score_drop = if self.best_move.is_some() {
            (i32::from(self.best_score) - i32::from(score)).clamp(0, 100) as u32
        } else {
            0
        };
        self.stable_iterations = if self.best_move == Some(best_move) {
            (self.stable_iterations + 1).min(5)
        } else {
            0
        };
        self.best_move = Some(best_move);
        self.best_score = score;

        let stability = 130 - 11 * self.stable_iterations;
        let effort =
            best_move_nodes_pct.map_or(100, |pct| (150 - u32::from(pct.min(100))).clamp(80, 120));
        let soft = Duration::from_millis(self.budget.soft_ms);
        let hard = Duration::from_millis(self.budget.hard_ms);
        let target =
            (soft.saturating_mul(stability * effort * (100 + score_drop)) / 1_000_000).min(hard);

        // Near the target, keep the completed PV if another iteration is
        // unlikely to fit in the remaining hard budget.
        elapsed < target
            && (elapsed < target.saturating_mul(3) / 4
                || elapsed.saturating_add(last_iteration.saturating_mul(3)) < hard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_budget_grows_near_the_next_time_control() {
        let unknown = TimeBudget::new(2000, 100, None, 15);
        let soon = TimeBudget::new(2000, 100, Some(5), 15);
        let next = TimeBudget::new(2000, 100, Some(1), 15);
        assert!(unknown.soft_ms < soon.soft_ms && soon.soft_ms < next.soft_ms);
        assert!(unknown.hard_ms < soon.hard_ms && soon.hard_ms < next.hard_ms);
        assert_eq!(next.hard_ms, 1985);
    }

    #[test]
    fn clock_budget_never_depends_on_an_unreceived_increment() {
        let empty = TimeBudget::new(10, 1000, None, 15);
        assert_eq!(empty.soft_ms, 0);
        assert_eq!(empty.hard_ms, 0);

        let short = TimeBudget::new(45, 1000, Some(2), 15);
        assert!(short.soft_ms <= short.hard_ms && short.hard_ms < 30);
    }

    #[test]
    fn unstable_move_or_score_gives_another_iteration() {
        let budget = TimeBudget {
            soft_ms: 100,
            hard_ms: 400,
        };
        let mut manager = TimeManager::new(budget);
        let elapsed = Duration::from_millis(95);
        let last_iteration = Duration::from_millis(15);

        for _ in 0..5 {
            assert!(manager.should_start_iteration(Duration::ZERO, last_iteration, 1, 100, None));
        }
        assert!(!manager.should_start_iteration(elapsed, last_iteration, 1, 100, None));
        assert!(manager.should_start_iteration(elapsed, last_iteration, 1, 0, None));
        assert!(manager.should_start_iteration(elapsed, last_iteration, 2, 0, None));
        assert!(!manager.should_start_iteration(
            Duration::from_millis(400),
            last_iteration,
            3,
            0,
            None
        ));
    }
}

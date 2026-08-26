#[cfg(debug_assertions)]
const SEARCH_RESPONSE_MARGIN: u64 = 50; //ms
#[cfg(not(debug_assertions))]
const SEARCH_RESPONSE_MARGIN: u64 = 15; //ms

const DEFAULT_STATIC_TIME: u64 = 5000;

///Config defining HOW we search. <br>
///Might be mutated in between searches to dynamically adjust behaviour.
pub struct SearchConfig {
    pub search_mode: SearchMode,
    pub quiescence: bool,
    pub log_diagnostics: bool,
    pub log_uci_diagnostics: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            search_mode: SearchMode::StaticTime(DEFAULT_STATIC_TIME),
            quiescence: true,
            log_diagnostics: false,
            log_uci_diagnostics: true,
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
        }
    }

}

pub enum SearchMode {
    StaticDepth(usize),
    StaticTime(u64), //ms
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
        let time_for_move = (time_left / 20).saturating_add(inc);
        return Self::StaticTime(time_for_move.saturating_sub(SEARCH_RESPONSE_MARGIN));
    }
}

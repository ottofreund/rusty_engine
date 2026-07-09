const DEFAULT_STATIC_TIME: f32 = 2000.0;
const DEFAULT_STATIC_DEPTH: usize = 8;

const SEARCH_RESPONSE_MARGIN: u64 = 50; //ms

///Config defining HOW we search. <br>
///Might be mutated in between searches to dynamically adjust behaviour.
pub struct SearchConfig {
    pub search_mode: SearchMode,
    pub quiescence: bool,
    pub log_diagnostics: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            search_mode: SearchMode::StaticTime(5000),
            quiescence: true,
            log_diagnostics: false,
        }
    }
}

impl SearchConfig {

    pub fn with_d(d: usize) -> Self {
        Self {
            search_mode: SearchMode::StaticDepth(d),
            quiescence: true,
            log_diagnostics: false,
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
        let time_per_move = (time_left / 30).saturating_add(inc);
        return Self::StaticTime(time_per_move.saturating_sub(SEARCH_RESPONSE_MARGIN));
    }
}

const DEFAULT_STATIC_TIME: f32 = 2000.0;

///Config defining HOW we search. <br>
///Might be mutated in between searches to dynamically adjust behaviour.
pub struct SearchConfig {
    pub search_mode: SearchMode,
    pub quiescence: bool,
    pub log_performance: bool
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self { search_mode: SearchMode::StaticTime(DEFAULT_STATIC_TIME), quiescence: true, log_performance: false }
    }
}

impl SearchConfig {
    fn with_d(d: usize) -> Self {
        Self {search_mode: SearchMode::StaticDepth(d), quiescence: true, log_performance: false}
    }
}

pub enum SearchMode {
    StaticDepth(usize),
    StaticTime(f32) //ms
}
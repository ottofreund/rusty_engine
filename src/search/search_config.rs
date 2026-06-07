const DEFAULT_STATIC_TIME: f32 = 2000.0;
const DEFAULT_STATIC_DEPTH: usize = 8;

///Config defining HOW we search. <br>
///Might be mutated in between searches to dynamically adjust behaviour.
pub struct SearchConfig {
    pub search_mode: SearchMode,
    pub quiescence: bool,
    pub log_performance: bool
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self { search_mode: SearchMode::StaticDepth(DEFAULT_STATIC_DEPTH), quiescence: true, log_performance: false }
    }
}

impl SearchConfig {
    fn with_d(d: usize) -> Self {
        Self {search_mode: SearchMode::StaticDepth(d), quiescence: true, log_performance: false}
    }

    fn with_performance_logging() -> Self {
        Self {search_mode: SearchMode::StaticDepth(DEFAULT_STATIC_DEPTH), quiescence: true, log_performance: true}
    }

}

pub enum SearchMode {
    StaticDepth(usize),
    StaticTime(f32) //ms
}
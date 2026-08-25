mod common;

use common::{TestEngine, MULTITHREADED};
use rusty_engine::{
    repr::_move::NULL_MOVE,
    search::{search_config::SearchMode, searcher::Searcher},
    utils::fen_tool::DEFAULT_FEN,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc, Barrier, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

const EXTERNAL_TIME_LIMIT_MS: u64 = 250;
const TIMING_TEST_ATTEMPTS: usize = 3;
const TACTICAL_FEN: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ";
const ENDGAME_FEN: &str = "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1";
const PINNED_PIECES_FEN: &str = "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1";
const PROMOTION_FEN: &str = "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8";
static TIMING_TEST_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy)]
struct TimedSearchCase {
    name: &'static str,
    fen: &'static str,
    prepared_depth: usize,
}

const TIMED_SEARCH_CASES: [TimedSearchCase; 5] = [
    TimedSearchCase {
        name: "starting position",
        fen: DEFAULT_FEN,
        prepared_depth: 2,
    },
    TimedSearchCase {
        name: "tactical middlegame",
        fen: TACTICAL_FEN,
        prepared_depth: 3,
    },
    TimedSearchCase {
        name: "pinned-pieces middlegame",
        fen: PINNED_PIECES_FEN,
        prepared_depth: 4,
    },
    TimedSearchCase {
        name: "promotion tactic",
        fen: PROMOTION_FEN,
        prepared_depth: 5,
    },
    TimedSearchCase {
        name: "sparse rook endgame",
        fen: ENDGAME_FEN,
        prepared_depth: 6,
    },
];

#[derive(Debug)]
struct TimingMeasurement {
    case_name: &'static str,
    attempt: usize,
    prepared_depth: usize,
    interrupted_depth: usize,
    elapsed: Duration,
    nodes: u64,
}

fn timed_searcher(
    engine: &TestEngine,
    fen: &str,
    search_mode: SearchMode,
    quiescence: bool,
) -> Searcher {
    let pos = engine.position(fen);
    let mut searcher = Searcher::from(&pos, MULTITHREADED);
    searcher.search_config.search_mode = search_mode;
    searcher.search_config.quiescence = quiescence;
    searcher.search_config.log_diagnostics = false;
    searcher.search_config.log_uci_diagnostics = false;
    searcher
}

fn static_search_budget(time_limit_ms: u64) -> Duration {
    match SearchMode::static_time_with_margin(time_limit_ms) {
        SearchMode::StaticTime(search_time_ms) => Duration::from_millis(search_time_ms),
        SearchMode::StaticDepth(_) => unreachable!("static time constructor returned a depth"),
    }
}

fn prepare_searcher(engine: &TestEngine, case: TimedSearchCase) -> Searcher {
    let mut searcher = timed_searcher(
        engine,
        case.fen,
        SearchMode::StaticDepth(case.prepared_depth),
        true,
    );
    searcher.start_search(&engine.move_gen, &engine.zobrist, None);

    let search_data = &searcher.search_data[0];
    let pv_depth = search_data.pv[..case.prepared_depth]
        .iter()
        .position(|mov| *mov == NULL_MOVE)
        .unwrap_or(case.prepared_depth);
    assert_eq!(
        pv_depth, case.prepared_depth,
        "{} did not produce a full depth-{} principal variation",
        case.name, case.prepared_depth
    );

    searcher
}

fn assert_search_stops_within<F>(description: &str, limit: Duration, mut run_search: F)
where
    F: FnMut() -> (Duration, u64),
{
    let measurements: Vec<(Duration, u64)> =
        (0..TIMING_TEST_ATTEMPTS).map(|_| run_search()).collect();
    let attempts_within_limit = measurements
        .iter()
        .filter(|(elapsed, _)| *elapsed <= limit)
        .count();

    assert!(
        attempts_within_limit == TIMING_TEST_ATTEMPTS,
        "{description} met {limit:?} on only {attempts_within_limit}/{TIMING_TEST_ATTEMPTS} attempts; every attempt must finish within the limit; (elapsed, nodes): {measurements:?}"
    );
}

#[test]
fn static_timed_search_uses_budget_across_positions_and_depths() {
    let _timing_guard = TIMING_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let engine = TestEngine::new();
    let time_limit = Duration::from_millis(EXTERNAL_TIME_LIMIT_MS);
    let search_budget = static_search_budget(EXTERNAL_TIME_LIMIT_MS);
    let response_margin = time_limit
        .checked_sub(search_budget)
        .expect("static search budget must not exceed its external time limit");
    let mut measurements = Vec::with_capacity(TIMED_SEARCH_CASES.len() * TIMING_TEST_ATTEMPTS);

    for case in TIMED_SEARCH_CASES {
        for attempt in 1..=TIMING_TEST_ATTEMPTS {
            let mut searcher = prepare_searcher(&engine, case);
            let nodes_before = searcher.search_data[0].cumul_positions_searched;
            searcher.search_config.search_mode =
                SearchMode::static_time_with_margin(EXTERNAL_TIME_LIMIT_MS);
            let kill_switch = Arc::new(AtomicBool::new(false));

            let start = Instant::now();
            searcher.start_search(&engine.move_gen, &engine.zobrist, Some(kill_switch));
            let elapsed = start.elapsed();
            let search_data = &searcher.search_data[0];
            let nodes = search_data
                .cumul_positions_searched
                .checked_sub(nodes_before)
                .expect("cumulative node count must not decrease");
            let interrupted_depth = search_data.pv_ply_indices[1];

            assert!(
                searcher.collect_best_move().is_some(),
                "{} did not complete a root move after searching {nodes} timed nodes",
                case.name
            );
            assert!(
                interrupted_depth > case.prepared_depth,
                "{} did not advance beyond prepared depth {}",
                case.name,
                case.prepared_depth
            );

            measurements.push(TimingMeasurement {
                case_name: case.name,
                attempt,
                prepared_depth: case.prepared_depth,
                interrupted_depth,
                elapsed,
                nodes,
            });
        }
    }

    let overruns: Vec<&TimingMeasurement> = measurements
        .iter()
        .filter(|measurement| measurement.elapsed > time_limit)
        .collect();
    assert!(
        overruns.is_empty(),
        "static timed search exceeded its external {time_limit:?} limit: {overruns:#?}"
    );

    let early_returns: Vec<&TimingMeasurement> = measurements
        .iter()
        .filter(|measurement| measurement.elapsed < search_budget)
        .collect();
    assert!(
        early_returns.is_empty(),
        "static timed search returned before using its {search_budget:?} internal budget: {early_returns:#?}"
    );

    let mut min_unspent = time_limit;
    let mut max_unspent = Duration::ZERO;
    let mut total_unspent = Duration::ZERO;
    for measurement in &measurements {
        let unspent = time_limit - measurement.elapsed;
        min_unspent = min_unspent.min(unspent);
        max_unspent = max_unspent.max(unspent);
        total_unspent += unspent;
        println!(
            "timed search {:>25} #{}/{}: prepared depth {} -> interrupted depth {}, {:>7.3} ms unspent ({:>7.3} ms elapsed, {} nodes)",
            measurement.case_name,
            measurement.attempt,
            TIMING_TEST_ATTEMPTS,
            measurement.prepared_depth,
            measurement.interrupted_depth,
            unspent.as_secs_f64() * 1_000.0,
            measurement.elapsed.as_secs_f64() * 1_000.0,
            measurement.nodes,
        );
    }

    let average_unspent = total_unspent / measurements.len() as u32;
    println!(
        "static timed search headroom across {} samples with a {:.3} ms response margin: min {:.3} ms, average {:.3} ms, max {:.3} ms",
        measurements.len(),
        response_margin.as_secs_f64() * 1_000.0,
        min_unspent.as_secs_f64() * 1_000.0,
        average_unspent.as_secs_f64() * 1_000.0,
        max_unspent.as_secs_f64() * 1_000.0,
    );
}

fn kill_switch_response_time(
    engine: &TestEngine,
    fen: &str,
    search_mode: SearchMode,
    quiescence: bool,
) -> (Duration, u64) {
    let mut searcher = timed_searcher(engine, fen, search_mode, quiescence);
    let kill_switch = Arc::new(AtomicBool::new(false));
    let kill_switch_setter = kill_switch.clone();
    let setter_ready = Arc::new(Barrier::new(2));
    let setter_barrier = setter_ready.clone();
    let setter = thread::spawn(move || {
        setter_barrier.wait();
        thread::sleep(Duration::from_millis(100));
        let kill_requested_at = Instant::now();
        kill_switch_setter.store(true, Relaxed);
        kill_requested_at
    });

    setter_ready.wait();
    searcher.start_search(&engine.move_gen, &engine.zobrist, Some(kill_switch.clone()));
    let stopped_at = Instant::now();
    let kill_requested_at = setter.join().expect("kill-switch setter panicked");
    let response_time = stopped_at
        .checked_duration_since(kill_requested_at)
        .expect("search stopped before the kill request");

    assert!(kill_switch.load(Relaxed));
    assert!(searcher.collect_best_move().is_some());
    assert_eq!(searcher.search_data[0].positions_searched, 0);
    assert_eq!(searcher.search_data[0].ab_cutoffs, 0);
    assert_eq!(searcher.search_data[0].stand_pat_cutoffs, 0);
    assert_eq!(searcher.search_data[0].sel_depth, 0);

    (
        response_time,
        searcher.search_data[0].cumul_positions_searched,
    )
}

#[test]
fn static_timed_search_observes_kill_switch_within_response_limit() {
    let _timing_guard = TIMING_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let engine = TestEngine::new();
    let time_limit = Duration::from_millis(EXTERNAL_TIME_LIMIT_MS);
    let response_limit = time_limit
        .checked_sub(static_search_budget(EXTERNAL_TIME_LIMIT_MS))
        .expect("search must reserve a response margin");

    assert_search_stops_within("mid-search kill switch", response_limit, || {
        kill_switch_response_time(&engine, DEFAULT_FEN, SearchMode::StaticTime(2000), false)
    });
}

#[test]
fn static_depth_search_observes_kill_switch_within_response_limit() {
    let _timing_guard = TIMING_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let engine = TestEngine::new();
    let time_limit = Duration::from_millis(EXTERNAL_TIME_LIMIT_MS);
    let response_limit = time_limit
        .checked_sub(static_search_budget(EXTERNAL_TIME_LIMIT_MS))
        .expect("search must reserve a response margin");

    assert_search_stops_within("fixed-depth kill switch", response_limit, || {
        kill_switch_response_time(&engine, TACTICAL_FEN, SearchMode::StaticDepth(6), true)
    });
}

mod common;

use common::{TestEngine, MULTITHREADED};
use rusty_engine::{
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
static TIMING_TEST_LOCK: Mutex<()> = Mutex::new(());

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
fn static_timed_search_returns_within_external_time_limit() {
    let _timing_guard = TIMING_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let engine = TestEngine::new();
    let cases = [
        ("starting position", DEFAULT_FEN, false),
        (
            "tactical position with quiescence",
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ",
            true,
        ),
    ];
    let time_limit = Duration::from_millis(EXTERNAL_TIME_LIMIT_MS);
    let search_budget = static_search_budget(EXTERNAL_TIME_LIMIT_MS);

    for (name, fen, quiescence) in cases {
        assert_search_stops_within(name, time_limit, || {
            let mut searcher = timed_searcher(
                &engine,
                fen,
                SearchMode::static_time_with_margin(EXTERNAL_TIME_LIMIT_MS),
                quiescence,
            );
            let kill_switch = Arc::new(AtomicBool::new(false));

            let start = Instant::now();
            searcher.start_search(&engine.move_gen, &engine.zobrist, Some(kill_switch));
            let elapsed = start.elapsed();
            let nodes = searcher.search_data[0].cumul_positions_searched;

            assert!(
                elapsed >= search_budget,
                "{name} returned before using its {search_budget:?} search budget: took {elapsed:?} after searching {nodes} nodes"
            );
            assert!(
                searcher.collect_best_move().is_some(),
                "{name} did not complete a root move after searching {nodes} nodes"
            );

            (elapsed, nodes)
        });
    }
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
        .expect("static timed search must reserve a response margin");

    assert_search_stops_within("mid-search kill switch", response_limit, || {
        let mut searcher =
            timed_searcher(&engine, DEFAULT_FEN, SearchMode::StaticTime(2000), false);
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
        searcher.start_search(&engine.move_gen, &engine.zobrist, Some(kill_switch));
        let stopped_at = Instant::now();
        let kill_requested_at = setter.join().expect("kill-switch setter panicked");
        let response_time = stopped_at
            .checked_duration_since(kill_requested_at)
            .expect("static timed search stopped before the kill request");

        (
            response_time,
            searcher.search_data[0].cumul_positions_searched,
        )
    });
}

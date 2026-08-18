mod common;

use common::TestEngine;
use rusty_engine::{
    repr::_move,
    search::{search_config::SearchMode, searcher::Searcher},
    utils::fen_tool::DEFAULT_FEN,
};
use std::{
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, Instant},
};

use crate::common::BENCH_MULTITHREADED;

const DEPTH: usize = 6;
const CONSECUTIVE_SEARCH_REPS: usize = 2;
const SWITCH_OVERHEAD_BENCH_REPS: usize = 4;
const STATIC_TIME_MS: u64 = 3000;
const NON_QUIESCENCE_SEARCH_TIME_MS: u64 = 250;
const SEARCH_CASES: [(&str, usize); 5] = [
    (DEFAULT_FEN, DEPTH),
    (
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ",
        6,
    ),
    ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 6),
    (
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        6,
    ),
    (
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        6,
    ),
];
const STATIC_TIME_SEARCH_CASES: [&str; 3] = [
    DEFAULT_FEN,
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ",
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
];

#[test]
#[ignore = "benchmark"]
fn static_depth_search_benchmark() {
    let engine = TestEngine::new();
    let mut total_positions = 0;
    let mut total_time = 0.0;

    for (fen, depth) in SEARCH_CASES {
        let (positions, time) = search_benchmark_pos(&engine, fen, depth, 1);
        total_positions += positions;
        total_time += time.as_secs_f64();
    }

    let nodes_per_second = if total_time == 0.0 {
        0.0
    } else {
        total_positions as f64 / total_time
    };

    println!(
        "\n\nstatic depth nodes per second: {:.0}, total time: {:.3} seconds, total nodes: {}\n\n",
        nodes_per_second, total_time, total_positions
    );
}

#[test]
#[ignore = "benchmark"]
fn static_depth_kill_switch_overhead_benchmark() {
    let engine = TestEngine::new();
    let mut without_switch_nodes = 0;
    let mut without_switch_time = Duration::ZERO;
    let mut with_switch_nodes = 0;
    let mut with_switch_time = Duration::ZERO;

    for (case_idx, (fen, depth)) in SEARCH_CASES.into_iter().enumerate() {
        for rep in 0..SWITCH_OVERHEAD_BENCH_REPS {
            let (without_switch, with_switch) = if (case_idx + rep) % 2 == 0 {
                (
                    fixed_depth_search_once(&engine, fen, depth, None),
                    fixed_depth_search_once(
                        &engine,
                        fen,
                        depth,
                        Some(Arc::new(AtomicBool::new(false))),
                    ),
                )
            } else {
                let with_switch = fixed_depth_search_once(
                    &engine,
                    fen,
                    depth,
                    Some(Arc::new(AtomicBool::new(false))),
                );
                let without_switch = fixed_depth_search_once(&engine, fen, depth, None);
                (without_switch, with_switch)
            };

            assert_eq!(without_switch.0, with_switch.0);
            without_switch_nodes += without_switch.0;
            without_switch_time += without_switch.1;
            with_switch_nodes += with_switch.0;
            with_switch_time += with_switch.1;
        }
    }

    assert_eq!(without_switch_nodes, with_switch_nodes);
    let without_switch_nps =
        without_switch_nodes as f64 / without_switch_time.as_secs_f64();
    let with_switch_nps = with_switch_nodes as f64 / with_switch_time.as_secs_f64();
    let elapsed_overhead =
        (with_switch_time.as_secs_f64() / without_switch_time.as_secs_f64() - 1.0) * 100.0;

    println!(
        "\n\nstatic depth without switch: {:.0} NPS ({} nodes in {:.3}s)\nstatic depth with inactive switch: {:.0} NPS ({} nodes in {:.3}s)\nelapsed-time difference: {:+.2}%\n",
        without_switch_nps,
        without_switch_nodes,
        without_switch_time.as_secs_f64(),
        with_switch_nps,
        with_switch_nodes,
        with_switch_time.as_secs_f64(),
        elapsed_overhead,
    );
}

#[test]
#[ignore = "benchmark"]
fn consecutive_search_benchmark() {
    let engine = TestEngine::new();
    let mut total_positions = 0;
    let mut total_time = Duration::ZERO;

    for (fen, _depth) in SEARCH_CASES {
        println!(
            "benchmarking {} consecutive searches at depth {} from {}",
            CONSECUTIVE_SEARCH_REPS, 5, fen
        );
        let (positions, time) = search_benchmark_pos(&engine, fen, 5, CONSECUTIVE_SEARCH_REPS);
        println!(
            "{} consecutive searches took {:.3} seconds and searched {} nodes",
            CONSECUTIVE_SEARCH_REPS,
            time.as_secs_f64(),
            positions
        );

        total_positions += positions;
        total_time += time;
    }

    let total_seconds = total_time.as_secs_f64();
    let nodes_per_second = if total_seconds == 0.0 {
        0.0
    } else {
        total_positions as f64 / total_seconds
    };

    println!(
        "\n\nconsecutive search nodes per second: {:.0}, total time: {:.3} seconds, total nodes: {}\n\n",
        nodes_per_second, total_seconds, total_positions
    );
}

#[test]
#[ignore = "benchmark"]
fn static_timed_search_benchmark() {
    let engine = TestEngine::new();
    let mut total_positions = 0;
    let mut total_time = Duration::ZERO;

    for fen in STATIC_TIME_SEARCH_CASES {
        let (positions, time) =
            static_timed_search_benchmark_pos(&engine, fen, STATIC_TIME_MS, true);
        println!(
            "static timed search at {}ms from {} took {:.3} seconds and searched {} positions",
            STATIC_TIME_MS,
            fen,
            time.as_secs_f64(),
            positions
        );

        total_positions += positions;
        total_time += time;
    }

    let total_seconds = total_time.as_secs_f64();
    let nodes_per_second = if total_seconds == 0.0 {
        0.0
    } else {
        total_positions as f64 / total_seconds
    };

    println!(
        "\n\nstatic timed search nodes per second: {:.0}, total time: {:.3} seconds, total positions: {}\n\n",
        nodes_per_second, total_seconds, total_positions
    );
}

#[test]
#[ignore = "benchmark"]
fn non_quiescence_search_benchmark() {
    let engine = TestEngine::new();
    let (nodes, elapsed) = static_timed_search_benchmark_pos(
        &engine,
        DEFAULT_FEN,
        NON_QUIESCENCE_SEARCH_TIME_MS,
        false,
    );
    let elapsed_secs = elapsed.as_secs_f64();
    let nps = nodes as f64 / elapsed_secs;

    println!(
        "\n\nnon-quiescence search: {:.0} NPS ({} nodes in {:.3} seconds)\n\n",
        nps, nodes, elapsed_secs
    );

    assert!(nodes > 0);
}

// Runs one fresh fixed-depth search without including switch construction in the timing.
fn fixed_depth_search_once(
    engine: &TestEngine,
    fen: &str,
    depth: usize,
    kill_switch: Option<Arc<AtomicBool>>,
) -> (u64, Duration) {
    let pos = engine.position(fen);
    let mut searcher = Searcher::from(&pos, BENCH_MULTITHREADED);
    searcher.search_config.search_mode = SearchMode::StaticDepth(depth);
    searcher.search_config.log_diagnostics = false;
    searcher.search_config.log_uci_diagnostics = false;

    let time_took = benchmark(|| {
        searcher.start_search(&engine.move_gen, &engine.zobrist, kill_switch);
    });

    let total_positions = if searcher.multithreaded {
        searcher
            .search_data
            .iter()
            .map(|data| data.cumul_positions_searched)
            .sum()
    } else {
        searcher.search_data[0].cumul_positions_searched
    };
    (total_positions, time_took)
}

// Returns (searched nodes, time taken).
// Supports making searched move and searching again with **reps**
fn search_benchmark_pos(
    engine: &TestEngine,
    fen: &str,
    depth: usize,
    reps: usize,
) -> (u64, Duration) {
    let mut pos = engine.position(fen);
    let mut searcher = Searcher::from(&pos, BENCH_MULTITHREADED);
    searcher.search_config.search_mode = SearchMode::StaticDepth(depth);

    let mut total_time_took = Duration::ZERO;
    for rep in 0..reps {
        let time_took: Duration = benchmark(|| {
            searcher.start_search(&engine.move_gen, &engine.zobrist, None);
        });
        total_time_took += time_took;
        let best_move: Option<u32> = searcher.collect_best_move();
        match best_move {
            Some(m) => {
                println!(
                    "search {}/{} took {:.3} seconds, best move: {}",
                    rep + 1,
                    reps,
                    time_took.as_secs_f64(),
                    _move::to_string(m, false)
                );
                pos.make_move(m, false, false, false, &engine.move_gen, &engine.zobrist);
                searcher.sync_new_move(&pos, Some(m));
            }
            None => {
                println!("game ended, stopping after search {}", rep + 1);
                break;
            }
        }
    }

    if searcher.multithreaded {
        let total_positions = searcher
            .search_data
            .iter()
            .map(|data| data.cumul_positions_searched)
            .sum();
        (total_positions, total_time_took)
    } else {
        (
            searcher.search_data[0].cumul_positions_searched,
            total_time_took,
        )
    }
}

// Returns (searched positions, time taken).
fn static_timed_search_benchmark_pos(
    engine: &TestEngine,
    fen: &str,
    time_ms: u64,
    quiescence: bool,
) -> (u64, Duration) {
    let pos = engine.position(fen);
    let mut searcher = Searcher::from(&pos, BENCH_MULTITHREADED);
    searcher.search_config.search_mode = SearchMode::StaticTime(time_ms);
    searcher.search_config.quiescence = quiescence;
    searcher.search_config.log_diagnostics = false;

    let kill_switch = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    let time_took = benchmark(|| {
        searcher.start_search(&engine.move_gen, &engine.zobrist, Some(kill_switch.clone()));
    });

    match searcher.collect_best_move() {
        Some(m) => println!("best move: {}", _move::to_string(m, false)),
        None => println!("game ended"),
    }

    if searcher.multithreaded {
        let total_positions = searcher
            .search_data
            .iter()
            .map(|data| data.cumul_positions_searched)
            .sum();
        (total_positions, time_took)
    } else {
        (searcher.search_data[0].cumul_positions_searched, time_took)
    }
}

fn benchmark<F: FnOnce()>(f: F) -> Duration {
    let start = Instant::now();
    f();
    start.elapsed()
}

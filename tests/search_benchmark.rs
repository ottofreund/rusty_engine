mod common;

use common::TestEngine;
use rusty_engine::{
    repr::_move,
    search::{search_config::SearchMode, searcher::Searcher},
    utils::fen_tool::DEFAULT_FEN,
};
use std::time::{Duration, Instant};

const DEPTH: usize = 8;
const CONSECUTIVE_SEARCH_REPS: usize = 2;
const SEARCH_CASES: [(&str, usize); 6] = [
    (DEFAULT_FEN, DEPTH),
    (
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ",
        7,
    ),
    (
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ",
        7,
    ),
    ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 7),
    (
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        7,
    ),
    (
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        7,
    ),
];

#[test]
#[ignore = "benchmark"]
fn search_benchmark() {
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
        "nodes per second: {:.0}, total time: {:.3} seconds, total nodes: {}",
        nodes_per_second, total_time, total_positions
    );
}

#[test]
#[ignore = "benchmark"]
fn consecutive_search_benchmark() {
    let engine = TestEngine::new();
    let mut total_positions = 0;
    let mut total_time = Duration::ZERO;

    for (fen, depth) in SEARCH_CASES {
        println!(
            "benchmarking {} consecutive searches at depth {} from {}",
            CONSECUTIVE_SEARCH_REPS, depth, fen
        );
        let (positions, time) = search_benchmark_pos(&engine, fen, depth, CONSECUTIVE_SEARCH_REPS);
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
        "consecutive search nodes per second: {:.0}, total time: {:.3} seconds, total nodes: {}",
        nodes_per_second, total_seconds, total_positions
    );
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
    let mut searcher = Searcher::from(&pos);
    searcher.search_config.search_mode = SearchMode::StaticDepth(depth);

    let mut total_time_took = Duration::ZERO;
    for rep in 0..reps {
        let time_took: Duration = benchmark(|| {
            searcher.start_search(&engine.move_gen, &engine.zobrist);
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
                pos.make_move(m, false, false, &engine.move_gen, &engine.zobrist);
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

fn benchmark<F: FnOnce()>(f: F) -> Duration {
    let start = Instant::now();
    f();
    start.elapsed()
}

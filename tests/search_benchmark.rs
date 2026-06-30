mod common;

use common::TestEngine;
use rusty_engine::{
    search::{search_config::SearchMode, searcher::Searcher},
    utils::fen_tool::DEFAULT_FEN,
};
use std::time::{Duration, Instant};

const DEPTH: usize = 8;
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
    let mut total_time = 0;

    for (fen, depth) in SEARCH_CASES {
        let (positions, time) = search_benchmark_pos(&engine, fen, depth);
        total_positions += positions;
        total_time += time;
    }

    let nodes_per_second = if total_time == 0 {
        0
    } else {
        total_positions / total_time
    };

    println!(
        "nodes per second: {}, total time: {} seconds, total nodes: {}",
        nodes_per_second, total_time, total_positions
    );
}

// Returns (searched nodes, time taken).
fn search_benchmark_pos(engine: &TestEngine, fen: &str, depth: usize) -> (u64, u64) {
    let pos = engine.position(fen);
    let mut searcher = Searcher::from(&pos);
    searcher.search_config.search_mode = SearchMode::StaticDepth(depth);

    let time_took = benchmark(|| {
        searcher.start_search(&engine.move_gen, &engine.zobrist);
    });

    if searcher.multithreaded {
        let total_positions = searcher
            .search_data
            .iter()
            .map(|data| data.cumul_positions_searched)
            .sum();
        (total_positions, time_took.as_secs())
    } else {
        (
            searcher.search_data[0].cumul_positions_searched,
            time_took.as_secs(),
        )
    }
}

fn benchmark<F: FnOnce()>(f: F) -> Duration {
    let start = Instant::now();
    f();
    start.elapsed()
}

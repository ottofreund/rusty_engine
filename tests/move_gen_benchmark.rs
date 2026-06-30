mod common;

use common::{TestEngine, PERFT_CASES};
use std::time::{Duration, Instant};

/// Benchmarking move generation moves/second.
#[test]
#[ignore = "benchmark"]
fn move_gen_benchmark() {
    let engine = TestEngine::new();
    let mut total_mps = 0.0;

    for case in PERFT_CASES {
        let mut pos = engine.position(case.fen);
        total_mps += perft_benchmark(|| engine.perft(case.depth, &mut pos));
    }

    println!(
        "average move gen speed: {:.2} million moves/second",
        total_mps / PERFT_CASES.len() as f32
    );
}

fn perft_benchmark<F: FnOnce() -> u32>(f: F) -> f32 {
    let start = Instant::now();
    let perft = f();
    let time_took: Duration = start.elapsed();

    (perft as f32 / 1e6) / time_took.as_secs_f32()
}

use std::time::{Duration, Instant};

use crate::{
    repr::{move_gen::MoveGen, position::Position},
    search::{search_config::SearchMode, searcher::Searcher},
    utils::zobrist::Zobrist,
};

pub const BENCH_DEPTH: usize = 6;

const BENCH_CASES: [&str; 5] = [
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
    "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
];

pub struct BenchResult {
    pub nodes: u64,
    pub nps: u64,
}

/// Runs Rusty's deterministic, single-threaded OpenBench workload.
pub fn run() -> BenchResult {
    let move_gen = MoveGen::init();
    let zobrist = Zobrist::default();
    let mut total_nodes = 0;
    let mut total_elapsed = Duration::ZERO;

    for fen in BENCH_CASES {
        let position = Position::from(fen, &move_gen, &zobrist)
            .expect("built-in benchmark position must be valid");
        let mut searcher = Searcher::from(&position, false);
        searcher.search_config.search_mode = SearchMode::StaticDepth(BENCH_DEPTH);
        searcher.search_config.log_diagnostics = false;
        searcher.search_config.log_uci_diagnostics = false;

        let start = Instant::now();
        searcher.start_search(&move_gen, &zobrist, None);
        total_elapsed += start.elapsed();
        total_nodes += searcher.search_data[0].cumul_positions_searched;
    }

    let nps = match total_elapsed.as_nanos() {
        0 => 0,
        elapsed_nanos => (u128::from(total_nodes) * 1_000_000_000 / elapsed_nanos) as u64,
    };

    BenchResult {
        nodes: total_nodes,
        nps,
    }
}

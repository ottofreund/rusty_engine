use rusty_engine::{repr::{move_gen::MoveGen, position::Position}, search::{search_config::SearchMode, searcher::Searcher}, utils::fen_tool::DEFAULT_FEN};
use std::time::{Duration, Instant};

const DEPTH: usize = 8;

#[test]
fn benchmark_default_pos() {
    search_benchmark_pos(DEFAULT_FEN, DEPTH);
}

#[test]
fn benchmark_kiwipete() {
    search_benchmark_pos("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ", DEPTH);
}


fn search_benchmark_pos(fen: &str, depth: usize) {
    let move_gen = MoveGen::init();
    let mut pos: Position = Position::position_with(fen, &move_gen).unwrap();
    let mut searcher = Searcher::from(&pos);
    searcher.search_config.search_mode = SearchMode::StaticDepth(depth);

    benchmark(|| {
        searcher.start_search(&move_gen);
    }, depth);
}

fn benchmark<F: FnOnce() -> ()>(f: F, depth: usize) {
    let start: Instant = Instant::now();
    let res: () = f();
    let time_took: Duration = start.elapsed();
    println!("search of depth {} took {} ms", depth, time_took.as_millis());
}
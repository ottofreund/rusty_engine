use rusty_engine::{repr::{move_gen::MoveGen, position::Position}, search::{search_config::SearchMode, searcher::Searcher}, utils::fen_tool::DEFAULT_FEN};
use std::time::{Duration, Instant};

const DEPTH: usize = 8;

#[test]
#[ignore = "benchmark"]
fn search_benchmark() {
    let mut total_positions: u64 = 0;
    let mut total_time: u64 = 0;
    //default pos
    let (positions, time) = search_benchmark_pos(DEFAULT_FEN, DEPTH);
    total_positions += positions;
    total_time += time;
    //kiwipete
    let (positions, time) = search_benchmark_pos("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ", 7);
    total_positions += positions;
    total_time += time;
    //edge case 2
    let (positions, time) = search_benchmark_pos("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ", 7);
    total_positions += positions;
    total_time += time;
    //edge case 3
    let (positions, time) = search_benchmark_pos("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 7);
    total_positions += positions;
    total_time += time;
    //edge case 4
    let (positions, time) = search_benchmark_pos("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1", 7);
    total_positions += positions;
    total_time += time;
    //edge case 5
    let (positions, time) = search_benchmark_pos("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8", 7);
    total_positions += positions;
    total_time += time;
    //log results
    println!("nodes per second: {}, total time: {} seconds, total nodes: {}", total_positions / total_time, total_time, total_positions);
}

//returns (searched nodes, time taken)
fn search_benchmark_pos(fen: &str, depth: usize) -> (u64, u64) {
    let move_gen = MoveGen::init();
    let pos: Position = Position::position_with(fen, &move_gen).unwrap();
    let mut searcher = Searcher::from(&pos);
    searcher.search_config.search_mode = SearchMode::StaticDepth(depth);

    let time_took: Duration = benchmark(|| {
        searcher.start_search(&move_gen);
    });

    if searcher.multithreaded {
        let mut total_positions: u64 = 0;
        for data in searcher.search_data {
            total_positions += data.cumul_positions_searched;
        }
        return (total_positions, time_took.as_secs());
    } else {
        return (searcher.search_data[0].cumul_positions_searched, time_took.as_secs());
    }

}

fn benchmark<F: FnOnce() -> ()>(f: F) -> Duration {
    let start: Instant = Instant::now();
    f();
    let time_took: Duration = start.elapsed();
    return time_took;
}
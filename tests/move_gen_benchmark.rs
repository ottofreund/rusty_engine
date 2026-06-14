use rusty_engine::repr::position::Position;
use rusty_engine::repr::move_gen::MoveGen;
use rusty_engine::utils::fen_tool::{DEFAULT_FEN};
use std::time::{Duration, Instant};

///Benchmarking move generation moves/per second.


#[test]
#[ignore = "benchmark"]
fn move_gen_benchmark() {
    let mut mms: f32 = 0.; //millions moves per second sum
    let runs: usize = 6;
    let move_gen = MoveGen::init();
    //default pos
    let mut pos: Position = Position::position_with(DEFAULT_FEN, &move_gen).unwrap();
    mms += perft_benchmark(|| {return go_perft(6, &mut pos, &move_gen);});
    //kiwipete
    pos = Position::position_with("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ", &move_gen).unwrap();
    mms += perft_benchmark(|| {return go_perft(5, &mut pos, &move_gen);});
    //edge case 3
    pos = Position::position_with("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", &move_gen).unwrap();
    mms += perft_benchmark(|| {return go_perft(7, &mut pos, &move_gen);});
    //edge case 4
    pos = Position::position_with("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1", &move_gen).unwrap();
    mms += perft_benchmark(|| {return go_perft(5, &mut pos, &move_gen);});
    //edge case 5
    pos = Position::position_with("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8", &move_gen).unwrap();
    mms += perft_benchmark(|| {return go_perft(5, &mut pos, &move_gen);});
    //edge case 6
    pos = Position::position_with("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10", &move_gen).unwrap();
    mms += perft_benchmark(|| {return go_perft(5, &mut pos, &move_gen);});

    println!("average move gen speed: {:.2} million moves/second", mms / runs as f32);
}





fn perft_benchmark<F: FnOnce() -> u32>(f: F) -> f32 {
    let start: Instant = Instant::now();
    let perft: u32 = f();
    let time_took: Duration = start.elapsed();
    //println!("perft took {} ms", time_took.as_millis());
    let mps = ((perft as f32 / 1e6) as f32) / (time_took.as_secs_f32());
    //println!("cranked {:.2} million moves/second", mps);
    return mps;
}


fn go_perft(target_depth: usize, pos: &mut Position, move_gen: &MoveGen) -> u32 {
    assert!(target_depth > 1);

    fn inner(d_left: usize, pos: &mut Position, move_gen: &MoveGen) -> usize {
        if d_left == 1 {
            return pos.legal_search_moves().len();
        } //else go deeper
        let mut from_here: usize = 0;
        let (s, e) = pos.search_move_bounds();
        for i in s..e {
            let mov: u32 = pos.move_arr[i];
            pos.make_move(mov, true, true, &move_gen);
            from_here += inner(d_left - 1, pos, move_gen);
            pos.unmake_move(mov);
        }
        return from_here;
    }

    return inner(target_depth, pos, move_gen) as u32;
}
use rusty_engine::repr::position::Position;
use rusty_engine::repr::move_gen::MoveGen;
use rusty_engine::repr::*;
use rusty_engine::utils::fen_tool::{DEFAULT_FEN};
use std::time::{Duration, Instant};

#[test]
fn default_pos_perft_correct() {
    let move_gen = MoveGen::init();
    let mut pos: Position = Position::position_with(DEFAULT_FEN, &move_gen).unwrap();
    //assert_eq!(go_perft(2, &mut pos), 400);
    //assert_eq!(go_perft(3, &mut pos), 8902)
    //assert_eq!(go_perft(4, &mut pos), 197281);
    //perft_logger(5, &mut pos, Some(4));
    //assert_eq!(go_perft(5, &mut pos), 4865609);
    //assert_eq!(go_perft(6, &mut pos, &move_gen), 119060324);
    
    perft_benchmark(|| {return go_perft(6, &mut pos, &move_gen);});
}

#[test]
fn edge_case_perft_2() { //"kiwipete" position
    let move_gen = MoveGen::init();
    let mut pos: Position = Position::position_with("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ", &move_gen).unwrap();
    //assert_eq!(go_perft(2, &mut pos), 2039);
    //pos.try_make_move(4, 6).unwrap();
    //pos.try_make_move(23, 14).unwrap();
    //perft_logger(2, &mut pos, Some(1));
    //assert_eq!(go_perft(3, &mut pos), 97862);
    //assert_eq!(go_perft(4, &mut pos), 4085603);
    assert_eq!(go_perft(5, &mut pos, &move_gen), 193690690);
    //perft_benchmark(|| {return go_perft(4, &mut pos, &move_gen);});
}

#[test]
fn edge_case_perft_3() {
    let move_gen = MoveGen::init();
    let mut pos: Position = Position::position_with("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", &move_gen).unwrap();
    //assert_eq!(go_perft(2, &mut pos), 191);
    //assert_eq!(go_perft(3, &mut pos), 2812)
    //assert_eq!(go_perft(4, &mut pos), 43238);
    //perft_logger(2, &mut pos, Some(1));
    //assert_eq!(go_perft(5, &mut pos), 674624);
    //assert_eq!(go_perft(6, &mut pos), 11030083);
    //assert_eq!(go_perft(7, &mut pos), 178633661);
    perft_benchmark(|| {return go_perft(7, &mut pos, &move_gen);});
}

#[test]
fn edge_case_perft_4() {
    let move_gen = MoveGen::init();
    let mut pos: Position = Position::position_with("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1", &move_gen).unwrap();
    //assert_eq!(go_perft(2, &mut pos), 264);
    //assert_eq!(go_perft(3, &mut pos), 9467)
    //assert_eq!(go_perft(4, &mut pos), 422333);
    //perft_logger(2, &mut pos, Some(1));
    assert_eq!(go_perft(5, &mut pos, &move_gen), 15833292);
}


#[test]
fn edge_case_perft_5() {
    let move_gen = MoveGen::init();
    let mut pos: Position = Position::position_with("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8", &move_gen).unwrap();
    //assert_eq!(go_perft(2, &mut pos), 1486);
    //assert_eq!(go_perft(3, &mut pos), 62379)
    //assert_eq!(go_perft(4, &mut pos), 2103487);
    //perft_logger(2, &mut pos, Some(1));
    assert_eq!(go_perft(5, &mut pos, &move_gen), 89941194);
}

#[test]
fn edge_case_perft_6() {
    let move_gen = MoveGen::init();
    let mut pos: Position = Position::position_with("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10", &move_gen).unwrap();
    //assert_eq!(go_perft(2, &mut pos), 2079);
    //assert_eq!(go_perft(3, &mut pos), 89890)
    //assert_eq!(go_perft(4, &mut pos), 3894594);
    //perft_logger(2, &mut pos, Some(1));
    assert_eq!(go_perft(5, &mut pos, &move_gen), 164075551);
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
            pos.make_move(mov, true, false, &move_gen);
            from_here += inner(d_left - 1, pos, move_gen);
            pos.unmake_move(mov);
        }
        return from_here;
    }

    return inner(target_depth, pos, move_gen) as u32;
}


///Helper for debugging to show distribution of moves one move deeper
fn perft_logger(depth: u32, pos: &mut Position, log_depth: Option<u32>, move_gen: &MoveGen) -> u32 {
    let found: u32;
    fn inner(d: u32, p: &mut Position, log_depth: Option<u32>, move_gen: &MoveGen) -> u32 {
        if d == 0 {
            return 1;
        } else {
            let mut perft_from_here: u32 = 0;
            p.legal_search_moves().to_vec().iter().for_each(|mov| {
                p.make_move(*mov, true, true, &move_gen);
                //println!("move_arr usage: {}", (p.move_arr_idx.last().copied().unwrap() as f32) / (p.move_arr.len() as f32));
                perft_from_here += inner(d - 1, p, log_depth, move_gen);
                p.unmake_move(*mov);
            });
            if log_depth.is_some() && log_depth.unwrap() == d {
                println!("After {:?} found {} positions\n", p.played_moves_stack.iter().map(|m| _move::to_string(*m)).collect::<Vec<String>>(), perft_from_here);
            } else if log_depth.is_none() {
                println!("After {:?} found {} positions\n", p.played_moves_stack.iter().map(|m| _move::to_string(*m)).collect::<Vec<String>>(), perft_from_here);
            }
            return perft_from_here;
        }
    }
    found = inner(depth, pos, log_depth, move_gen);
    println!("total found: {}", found);
    return found;
}

fn perft_benchmark<F: FnOnce() -> u32>(f: F) {
    let start: Instant = Instant::now();
    let perft: u32 = f();
    let time_took: Duration = start.elapsed();
    println!("perft took {} ms", time_took.as_millis());
    println!("cranked {:.2} million moves/second", ((perft as f32 / 1e6) as f32) / (time_took.as_secs_f32()))
}
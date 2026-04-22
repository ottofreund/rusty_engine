use rusty_engine::repr::game::Game;
use rusty_engine::repr::*;
use rusty_engine::utils::fen_tool::{DEFAULT_FEN};
use std::time::{Duration, Instant};

#[test]
fn default_pos_perft_correct() {
    let mut game: Game = Game::game_with(DEFAULT_FEN).unwrap();

    //assert_eq!(go_perft(2, &mut game), 400);
    //assert_eq!(go_perft(3, &mut game), 8902)
    //assert_eq!(go_perft(4, &mut game), 197281);
    //perft_logger(5, &mut game, Some(4));
    //assert_eq!(go_perft(5, &mut game), 4865609);
    //assert_eq!(go_perft(6, &mut game), 119060324);
    
    perft_benchmark(|| {return go_perft(6, &mut game);});
}

#[test]
fn edge_case_perft_2() { //"kiwipete" position
    let mut game: Game = Game::game_with("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ").unwrap();
    //assert_eq!(go_perft(2, &mut game), 2039);
    //game.try_make_move(4, 6).unwrap();
    //game.try_make_move(23, 14).unwrap();
    //perft_logger(2, &mut game, Some(1));
    //assert_eq!(go_perft(3, &mut game), 97862);
    //assert_eq!(go_perft(4, &mut game), 4085603);
    //assert_eq!(go_perft(5, &mut game), 193690690);
    perft_benchmark(|| {return go_perft(4, &mut game);});
}

#[test]
fn edge_case_perft_3() {
    let mut game: Game = Game::game_with("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").unwrap();
    //assert_eq!(go_perft(2, &mut game), 191);
    //assert_eq!(go_perft(3, &mut game), 2812)
    //assert_eq!(go_perft(4, &mut game), 43238);
    //perft_logger(2, &mut game, Some(1));
    //assert_eq!(go_perft(5, &mut game), 674624);
    //assert_eq!(go_perft(6, &mut game), 11030083);
    //assert_eq!(go_perft(7, &mut game), 178633661);
    perft_benchmark(|| {return go_perft(7, &mut game);});
}

#[test]
fn edge_case_perft_4() {
    let mut game: Game = Game::game_with("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1").unwrap();
    //assert_eq!(go_perft(2, &mut game), 264);
    //assert_eq!(go_perft(3, &mut game), 9467)
    //assert_eq!(go_perft(4, &mut game), 422333);
    //perft_logger(2, &mut game, Some(1));
    assert_eq!(go_perft(5, &mut game), 15833292);
}


#[test]
fn edge_case_perft_5() {
    let mut game: Game = Game::game_with("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8").unwrap();
    //assert_eq!(go_perft(2, &mut game), 1486);
    //assert_eq!(go_perft(3, &mut game), 62379)
    //assert_eq!(go_perft(4, &mut game), 2103487);
    //perft_logger(2, &mut game, Some(1));
    assert_eq!(go_perft(5, &mut game), 89941194);
}

#[test]
fn edge_case_perft_6() {
    let mut game: Game = Game::game_with("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10").unwrap();
    //assert_eq!(go_perft(2, &mut game), 2079);
    //assert_eq!(go_perft(3, &mut game), 89890)
    //assert_eq!(go_perft(4, &mut game), 3894594);
    //perft_logger(2, &mut game, Some(1));
    assert_eq!(go_perft(5, &mut game), 164075551);
}


fn go_perft(target_depth: usize, game: &mut Game) -> u32 {
    assert!(target_depth > 1);

    fn inner(d_left: usize, game: &mut Game) -> usize {
        if d_left == 1 {
            return game.legal_search_moves().len();
        } //else go deeper
        let mut from_here: usize = 0;
        let (s, e) = game.search_move_bounds();
        for i in s..e {
            let mov: u32 = game.move_arr[i];
            game.make_move(mov, true, false);
            from_here += inner(d_left - 1, game);
            game.unmake_move(mov);
        }
        return from_here;
    }

    return inner(target_depth, game) as u32;
}

//legacy, replaced by recursive for simplicity, performance around the same
fn go_perft_iterative(target_depth: usize, game: &mut Game) -> u32 {
    assert!(target_depth > 1);
    let mut found: usize = 0;
    //"pointer" to cur move idx of each ply, when higher ply covered increment lower ply
    let mut per_ply_idx: Vec<usize> = vec![0; target_depth + 1]; 
    let mut cur_ply: usize = 1;

    while cur_ply > 0 {
        let mv_idx: usize = game.move_arr_idx[cur_ply - 1] + per_ply_idx[cur_ply];
        if mv_idx == game.move_arr_idx[cur_ply] { //covered this ply
            if cur_ply == 1 {
                break;
            }
            for p in cur_ply..target_depth { //reset higher ply pre_ply_idx
                per_ply_idx[p] = 0;
            }
            let prev_ply_mv_idx: usize = game.move_arr_idx[cur_ply - 2] + per_ply_idx[cur_ply - 1] - 1;
            game.unmake_move(game.move_arr[prev_ply_mv_idx]); //unmake last made move (from prev ply)
            cur_ply -= 1;
            continue;
        } else { //not done with this ply
            game.make_move(game.move_arr[mv_idx], true, true);
            per_ply_idx[cur_ply] += 1;
            cur_ply += 1;
            if cur_ply == target_depth { //terminal cond
                found += game.move_arr_idx[cur_ply] - game.move_arr_idx[cur_ply - 1];
                cur_ply -= 1;
                game.unmake_move(game.move_arr[mv_idx]);
                continue;
            } //else we go deeper
        }
    }
    //println!("Found {}", found);
    return found as u32;
}

///Helper for debugging to show distribution of moves one move deeper
fn perft_logger(depth: u32, game: &mut Game, log_depth: Option<u32>) -> u32 {
    let found: u32;
    fn inner(d: u32, g: &mut Game, log_depth: Option<u32>) -> u32 {
        if d == 0 {
            return 1;
        } else {
            let mut perft_from_here: u32 = 0;
            g.legal_search_moves().to_vec().iter().for_each(|mov| {
                g.make_move(*mov, true, true);
                //println!("move_arr usage: {}", (g.move_arr_idx.last().copied().unwrap() as f32) / (g.move_arr.len() as f32));
                perft_from_here += inner(d - 1, g, log_depth);
                g.unmake_move(*mov);
            });
            if log_depth.is_some() && log_depth.unwrap() == d {
                println!("After {:?} found {} positions\n", g.played_moves_stack.iter().map(|m| _move::to_string(*m)).collect::<Vec<String>>(), perft_from_here);
            } else if log_depth.is_none() {
                println!("After {:?} found {} positions\n", g.played_moves_stack.iter().map(|m| _move::to_string(*m)).collect::<Vec<String>>(), perft_from_here);
            }
            return perft_from_here;
        }
    }
    found = inner(depth, game, log_depth);
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
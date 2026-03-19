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
    assert_eq!(go_perft(5, &mut game), 4865609)
    //assert_eq!(go_perft(6, &mut game), 119060324)
    
    //perft_benchmark(|| {go_perft_better(4, &mut game);});
    //perft_benchmark(|| {go_perft(5, &mut game);});
}

#[test]
fn kiwipete_edge_case_perft() {
    let mut game: Game = Game::game_with("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ").unwrap();

}

fn go_perft(target_depth: usize, game: &mut Game) -> u32 {
    assert!(target_depth > 1);
    let mut found: usize = 0;
    //"pointer" to cur move idx of each ply, when higher ply covered increment lower ply
    let mut per_ply_idx: Vec<usize> = vec![0; target_depth]; 
    let mut cur_ply: usize = 0;
    let mut reached_last_ply: bool = false;

    for i in 0..=target_depth {
        if game.move_arr_idx.len() < i {
            
        }
    }

    while per_ply_idx[0] < 1 {

        while !reached_last_ply {

            

            if cur_ply == target_depth {
                reached_last_ply = true;
            } else {
                cur_ply += 1;
            }
        }


        //println!("Made moves:\n{:?}", made_moves_stack.iter().map(|m| _move::to_string(*m)).collect::<Vec<String>>()); 
        if made_moves_stack.len() == target_depth as usize - 1 {
            found += game.legal_moves().len();
            let last_made_move: Option<u32> = made_moves_stack.last().copied();
            while move_stack.pop() != last_made_move {}; //pop all counted moves and last made move
            //now unmake last made on board and pop from made moves 
            game.unmake_move(made_moves_stack.pop().expect("made moves stack was empty"));
        } else if made_moves_stack.last().copied() == move_stack.last().copied() { //covered all subvariations after this move to desired depth, unmake move
            move_stack.pop();
            game.unmake_move(made_moves_stack.pop().expect("made moves stack was empty"));
        } else { //make move that is on top of stack
            let next_mov: u32 = move_stack.last().copied().expect("move stack never empty here");
            //println!("now playing: {}", _move::to_string(next_mov));
            game.make_move(next_mov);
            made_moves_stack.push(next_mov);
            //add all legal moves of resulting position to move stack
            game.legal_moves().iter().copied().for_each(|mov| {
                move_stack.push(mov);
            });
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
            g.legal_moves().to_vec().iter().for_each(|mov| {
                g.make_move(*mov);
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

fn perft_benchmark<F: FnOnce()>(f: F) {
    let start: Instant = Instant::now();
    f();
    let time_took: Duration = start.elapsed();
    println!("perft took {} ms", time_took.as_millis());
}
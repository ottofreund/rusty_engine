use std::{cmp::{Ordering, max}, collections::HashMap};
use std::time::{Duration, Instant};

use crate::{repr::{_move::{self, *}, move_gen::MoveGen, position::Position, types::{BLACK, NOF_PIECE_TYPES, WHITE}}, search::{eval::{Evaluator, MATE_EVAL, PRUNE_EVAL}, search_config::*, search_data::SearchData}};


pub const MAX_SEARCH_DEPTH: usize = 50;
const THREAD_COUNT: usize = 4;
const ALPHA_INIT: i32 = -1_000_000_000;
const BETA_INIT: i32 = 1_000_000_000;

const PROMOTION_SCORE: i32 = 1_000_000;
const LAST_TARGET_SCORE: i32 = 10_000;
const EATING_MULTIPLIER: i32 = 100;
const BASELINE_SCORE: i32 = 1000; //to avoid underflow

pub struct Searcher {
    pub positions: [ Position ; THREAD_COUNT ],
    pub search_data: [SearchData ; THREAD_COUNT],
    pub multithreaded: bool,
    pub search_config: SearchConfig,
    pub evaluator: Evaluator,
}


//minimax with alpha beta pruning, ran by iterative deepening
//search heuristics in ordering of moves
impl Searcher {

    pub fn from(pos: &Position) -> Searcher {
        let positions: [ Position ; THREAD_COUNT ] = std::array::from_fn(|_| {
            return (*pos).clone();
        });
        let search_data: [SearchData ; THREAD_COUNT] = std::array::from_fn(|_| {
            return SearchData::default();
        });
        return Self {
            positions, search_data, multithreaded: false, search_config: SearchConfig::default(), evaluator: Evaluator::default()
        }
    }

    
    pub fn start_search(&mut self, move_gen: &MoveGen) {
        if self.multithreaded {
            panic!("multithreaded search");
            //PRAGMA FOR LOOP HERE
            for i in 0..THREAD_COUNT {
                self.start_search_node(i, move_gen);
            }
        } else {
            self.start_search_node(0, move_gen);
        }
    }

    
    fn start_search_node(&mut self, idx: usize, move_gen: &MoveGen) {
        match self.search_config.search_mode {
            SearchMode::StaticDepth(d) => {
                self.search_static_d(idx, move_gen);
            },
            SearchMode::StaticTime(t) => {
                return;
            },
            _ => panic!("unknown search mode")
        }
        return;
    }


    ///alpha-beta pruned negamax algorithm with iterative deepening
    fn search_static_d(&mut self, idx: usize, move_gen: &MoveGen) {

        let pos: &mut Position = &mut self.positions[idx];
        let search_data: &mut SearchData = &mut self.search_data[idx];
        //cv: current variation, pv: primary variation
        fn inner(d: usize, target_d: usize, alpha: i32, beta: i32,  pv: &mut [u32], prev_pv: &[u32], pos: &mut Position, evaluator: &Evaluator, move_gen: &MoveGen, positions_searched: &mut u64, ab_cutoffs: &mut u64) -> i32 {
            *positions_searched += 1;
            let (s, e) = pos.search_move_bounds();
            if s == e { //mate or stalemate
                if pos.board.nof_checkers > 0 {
                    return -MATE_EVAL + d as i32; //sooner mate is better
                } else {
                    return 0; //stalemate
                }
            } else if d == target_d {
                return evaluator.eval(pos.board.pieces, pos.board.turn, pos.is_late_game());
            }
            
            let mut eval: i32 = i32::MIN;            
            
            let mut new_alpha: i32 = alpha;
            for i in s..e {
                let mut child_pv: [u32 ; MAX_SEARCH_DEPTH + 1] = [ NULL_MOVE ; MAX_SEARCH_DEPTH + 1 ];
                let mov: u32 = partial_selection_sort(&mut pos.move_arr[i..e], prev_pv[d], pos.last_target);
                pos.make_move(mov, true, false, move_gen);
                let mov_eval: i32 = -inner(d + 1, target_d, -beta, -new_alpha, &mut child_pv, prev_pv, pos, evaluator, move_gen, positions_searched, ab_cutoffs);
                pos.unmake_move(mov);
                
                if mov_eval > eval {
                    eval = mov_eval;
                    //child's pv becomes this node's pv
                    pv[d + 1..=target_d].copy_from_slice(&child_pv[d + 1..=target_d]);
                    pv[d] = mov;
                }

                new_alpha = max(new_alpha, mov_eval);

                if new_alpha >= beta {
                    *ab_cutoffs += 1;
                    return new_alpha;
                }

            }
            return eval;
        }
        //iterative deepening:
        let target_d: usize;
        match self.search_config.search_mode {
            SearchMode::StaticDepth(d) => {
                target_d = d;
            },
            SearchMode::StaticTime(_) => {
                panic!("Called search_static_d with static time config");
            }
        }
        let mut pv: [u32 ; MAX_SEARCH_DEPTH + 1] = [ NULL_MOVE ; MAX_SEARCH_DEPTH + 1 ];
        for i in 1..=target_d {
            let prev_pv: [u32 ; MAX_SEARCH_DEPTH + 1] = pv;
            pv.fill(NULL_MOVE);
            let eval: i32 = inner(0, i, ALPHA_INIT, BETA_INIT, &mut pv, &prev_pv, pos, &self.evaluator, move_gen, &mut search_data.positions_searched, &mut search_data.ab_cutoffs);
            println!("at depth: {}, eval: {}", i, eval);
            search_data.cumul_positions_searched += search_data.positions_searched;
            search_data.log_performance();
            search_data.reset_temp_performance_data();
        }
        
        search_data.pv_move.fill(NULL_MOVE);
        let mut i = 0;
        while i <= MAX_SEARCH_DEPTH {
            search_data.pv_move[i] = pv[i];
            if pv[i] == NULL_MOVE {
                break;
            }
            i += 1;
        }

        println!("got pv: {:?}", search_data.pv_move.map(|m| _move::to_string(m)));

        return;
    }

    
}


//k == 1, so "selection pick", in place
fn partial_selection_sort(move_arr_s: &mut [u32], pv_mv: u32, last_target: u32) -> u32 {
    if move_arr_s.len() == 0 {
        return NULL_MOVE;
    }
    let mut best_v: i32 = BASELINE_SCORE;
    let mut best_i: usize = 0;
    let mut best_m: u32 = NULL_MOVE;
    for i in 0..move_arr_s.len() {
        let mut cur_v: i32 = BASELINE_SCORE;
        let mov: u32= move_arr_s[i];
        if mov == pv_mv {
            best_i = i;
            best_m = mov;
            break;
        } 
        if _move::is_promotion(mov) {
            cur_v += PROMOTION_SCORE + _move::get_promoted_piece(mov) as i32;
        }
        if _move::get_target(mov) == last_target {
            cur_v += LAST_TARGET_SCORE;
        }
        if _move::is_eating(mov) {
            cur_v += EATING_MULTIPLIER * ((_move::eaten_piece(mov).unwrap() % NOF_PIECE_TYPES) as i32 - (_move::get_moved_piece(mov) % NOF_PIECE_TYPES) as i32) ;
        } 
        if cur_v > best_v {
            best_v = cur_v;
            best_i = i;
            best_m = mov;
        }
    }
    if best_m != NULL_MOVE {
        let t: u32 = move_arr_s[0];
        move_arr_s[0] = best_m;
        move_arr_s[best_i] = t;
    }
    return move_arr_s[0];
}
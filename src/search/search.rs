use std::{cmp::{Ordering, max}, collections::HashMap};

use crate::{repr::{_move::{self, *}, move_gen::MoveGen, position::Position, types::{BLACK, WHITE}}, search::{eval::{Evaluator, MATE_EVAL, PRUNE_EVAL}, search_config::*, search_data::SearchData}};


pub const MAX_SEARCH_DEPTH: usize = 50;
const THREAD_COUNT: usize = 4;
const AB_INIT: i32 = -1_000_000_000;

const PV_SCORE: u32 = 1_000_000;
const PROMOTION_SCORE: u32 = 500_000;
const LAST_TARGET_SCORE: u32 = 100_000;
const EATING_SCORE: u32 = 50;

pub struct Searcher {
    pub positions: [ Position ; THREAD_COUNT ],
    pub search_data: [SearchData ; THREAD_COUNT],
    pub multithreaded: bool,
    pub search_config: SearchConfig,
    pub evaluator: Evaluator,
}


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

    //minimax with alpha beta pruning, ran by iterative deepening
    //search heuristics in ordering of moves
    pub fn start_search(&self, pos: &mut Position, search_data: &mut SearchData) {
        if self.multithreaded {
            return;
        } else {
            match self.search_config.search_mode {
                SearchMode::StaticDepth(d) => {

                },
                SearchMode::StaticTime(t) => {

                },
                _ => panic!("unknown search mode")
            }
            return;
        }
    }



    ///alpha-beta pruned negamax algorithm with iterative deepening
    fn search_static_d(&self, pos: &mut Position, search_data: &mut SearchData, move_gen: &MoveGen) {

        //cv: current variation, pv: primary variation
        fn inner(d: usize, target_d: usize, alpha: i32, beta: i32, cv: &mut [u32], pv: &mut [u32], pos: &mut Position, evaluator: &Evaluator, move_gen: &MoveGen) -> i32 {
            if d == target_d {
                return evaluator.eval(pos.board.pieces, pos.board.turn, pos.is_late_game());
            }
            let mut eval: i32 = -MATE_EVAL;
            let mut chosen: u32 = NULL_MOVE;
            let (s, e) = pos.search_move_bounds();
            
            let mut new_alpha: i32 = alpha;
            let mut new_beta: i32 = beta;
            for i in s..e {
                //let mov: u32 = game.move_arr[i];
                let mov: u32 = partial_selection_sort(&mut pos.move_arr[i..e], pv[d], pos.last_target);
                pos.make_move(mov, true, false, move_gen);
                let mov_eval: i32 = -inner(d + 1, target_d, new_alpha, new_beta, cv, pv, pos, evaluator, move_gen);
                pos.unmake_move(mov);

                if pos.board.turn == WHITE {
                    new_alpha = max(new_alpha, mov_eval);
                } else {
                    new_beta = max(new_beta, mov_eval);
                }
                
                if pos.board.turn == WHITE && mov_eval >= -new_beta { //beta cutoff (black can force better earlier)
                    return mov_eval;
                } else if pos.board.turn == BLACK && mov_eval >= -new_alpha { //alpha cut off
                    return mov_eval
                }
                
                
                //cur variation wasn't pruned, so if in root, cur variation is a new primary variation
                if d == 0 {
                    cv[d] = mov;
                    let mut p: usize = 0;
                    while p < MAX_SEARCH_DEPTH { //copy to pv
                        let m: u32 = cv[p];
                        pv[p] = m;
                        if m == NULL_MOVE {
                            break;
                        }
                        p += 1;
                    }
                }
                if mov_eval > eval {
                    eval = mov_eval;
                    chosen = mov;
                }
            }
            cv[d] = chosen;
            if d == target_d {
                cv[d + 1] = NULL_MOVE;
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
        let mut cv: [u32 ; MAX_SEARCH_DEPTH + 1] = [ NULL_MOVE ; MAX_SEARCH_DEPTH + 1 ];
        let mut pv: [u32 ; MAX_SEARCH_DEPTH + 1] = [ NULL_MOVE ; MAX_SEARCH_DEPTH + 1 ];
        for i in 1..=target_d {
            let eval: i32 = inner(0, i, AB_INIT, AB_INIT, &mut cv, &mut pv, pos, &self.evaluator, move_gen);
            println!("at depth: {}, eval: {}", i, eval);
        }
        let mut i: usize = 0;
        let mut m: u32 = pv[i];
        search_data.pv_move[i] = m;
        while m != NULL_MOVE && i < MAX_SEARCH_DEPTH {
            i += 1;
            m = pv[i];
            search_data.pv_move[i] = m;
        }
        return;
    }

    

    
}

///in place DEPRECATED
fn order_moves(move_arr_s: &mut [u32], pv_mv: u32, last_target: u32) {
    move_arr_s.sort_by_key(|m| {
        let mov
        = *m;
        if mov == pv_mv {
            return PV_SCORE; 
        } else if _move::is_promotion(mov) {
            return PROMOTION_SCORE + _move::get_promoted_piece(mov);
        } else if _move::get_target(mov) == last_target {
            return LAST_TARGET_SCORE;
        } else if _move::is_eating(mov) {
            return (_move::eaten_piece(mov).unwrap() + EATING_SCORE) - _move::get_moved_piece(mov);
        } else {
            return 0;
        }
    })
}

//k == 1, so "selection pick", in place
fn partial_selection_sort(move_arr_s: &mut [u32], pv_mv: u32, last_target: u32) -> u32 {
    if move_arr_s.len() == 0 {
        return NULL_MOVE;
    }
    let mut best_v: u32 = 0;
    let mut best_i: usize = 0;
    let mut best_m: u32 = NULL_MOVE;
    for i in 0..move_arr_s.len() {
        let mov: u32= move_arr_s[i];
        if mov == pv_mv {
            best_i = i;
            best_m = mov;
            break;
        } else if _move::is_promotion(mov) && (PROMOTION_SCORE + _move::get_promoted_piece(mov)) > best_v {
            best_v = PROMOTION_SCORE + _move::get_promoted_piece(mov);
            best_i = i;
            best_m = mov;
        } else if _move::get_target(mov) == last_target && LAST_TARGET_SCORE > best_v {
            best_v = LAST_TARGET_SCORE;
            best_i = i;
            best_m = mov;
        } else if _move::is_eating(mov) {
            let v: u32 =  (_move::eaten_piece(mov).unwrap() + EATING_SCORE) - _move::get_moved_piece(mov);
            if v > best_v {
                best_v = v;
                best_i = i;
                best_m = mov;
            }
        } 
    }
    if best_m != NULL_MOVE {
        let t: u32 = move_arr_s[0];
        move_arr_s[0] = best_m;
        move_arr_s[best_i] = t;
    }
    return move_arr_s[0];
}
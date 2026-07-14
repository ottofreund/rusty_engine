use std::{cmp::max, sync::{Arc, atomic::{AtomicBool, Ordering::Relaxed}}, time::Instant};

use crate::{
    repr::{
        _move::{self, *},
        move_gen::MoveGen,
        position::Position,
        types::NOF_PIECE_TYPES,
    },
    search::{
        eval::{Evaluator, MATE_EVAL},
        search_config::*,
        search_data::SearchData,
    },
    utils::zobrist::Zobrist,
};

pub const MAX_SEARCH_DEPTH: usize = 50;
const THREAD_COUNT: usize = 4;
const ALPHA_INIT: i32 = -1_000_000_000;
const BETA_INIT: i32 = 1_000_000_000;
const EVAL_INIT: i32 = -1_000_000_000;
const EVAL_QUIT: i32 = 555_555_555;

const PROMOTION_SCORE: i32 = 1_000_000;
const LAST_TARGET_SCORE: i32 = 10_000;
const EATING_MULTIPLIER: i32 = 100;
const BASELINE_SCORE: i32 = 1000; //to avoid underflow

pub struct Searcher {
    pub positions: [Position; THREAD_COUNT],
    pub search_data: [SearchData; THREAD_COUNT],
    pub multithreaded: bool,
    pub search_config: SearchConfig,
    pub evaluator: Evaluator,
    last_sync_deviates_from_pv: bool,
}

//minimax with alpha beta pruning, ran by iterative deepening
//search heuristics in ordering of moves
impl Searcher {
    pub fn import_position(&mut self, pos: &Position, board_hash_history: Option<Vec<u64>>) {
        for i in 0..THREAD_COUNT {
            self.positions[i] = (*pos).clone();
        }
        self.search_data = std::array::from_fn(|_| {
            if let Some(bhh) = &board_hash_history {
                return SearchData::with_board_hash_history(pos, bhh.clone());
            } else {
                return SearchData::new(pos);
            }
        });
        self.last_sync_deviates_from_pv = true;
    }

    ///Both engine moves and user moves are synced
    /// In UCI mov is not always defined, may be just position
    pub fn sync_new_move(&mut self, new_pos: &Position, mov: Option<u32>) {
        self.last_sync_deviates_from_pv = match self.collect_best_move() {
            Some(bm) if mov.is_some() => bm != mov.unwrap(),
            _ => true,
        };
        if self.search_config.log_diagnostics {
            println!(
                "last sync deviates from pv: {}",
                self.last_sync_deviates_from_pv
            );
        }
        for i in 0..THREAD_COUNT {
            self.positions[i] = (*new_pos).clone();
            if mov.is_some() && _move::is_unrepeatable(mov.unwrap()) {
                self.search_data[i].board_hash_history.clear();
            }
            self.search_data[i]
                .board_hash_history
                .push(new_pos.board.zhash);

            if self.last_sync_deviates_from_pv {
                self.search_data[i].pv.fill(NULL_MOVE);
            } else {
                self.drop_pv_head(i);
            }
        }
    }

    pub fn from(pos: &Position) -> Searcher {
        let positions: [Position; THREAD_COUNT] = std::array::from_fn(|_| {
            return (*pos).clone();
        });
        let search_data: [SearchData; THREAD_COUNT] = std::array::from_fn(|_| {
            return SearchData::new(pos);
        });
        let search_config = SearchConfig::default();
        return Self {
            positions,
            search_data,
            multithreaded: false,
            search_config,
            evaluator: Evaluator::default(),
            last_sync_deviates_from_pv: true,
        };
    }

    pub fn start_search(&mut self, move_gen: &MoveGen, zobrist: &Zobrist, kill_switch: Option<Arc<AtomicBool>>) {
        if self.multithreaded {
            panic!("multithreaded search");
            //PRAGMA FOR LOOP HERE
            for i in 0..THREAD_COUNT {
                self.start_search_node(i, move_gen, zobrist, kill_switch.clone());
            }
        } else {
            self.start_search_node(0, move_gen, zobrist, kill_switch);
        }
    }

    fn start_search_node(&mut self, idx: usize, move_gen: &MoveGen, zobrist: &Zobrist, kill_switch: Option<Arc<AtomicBool>>) {
        match self.search_config.search_mode {
            SearchMode::StaticDepth(d) => {
                self.search_static_d(d, idx, move_gen, zobrist);
            }
            SearchMode::StaticTime(t) => {
                self.search_static_time(t, idx, move_gen, zobrist, kill_switch);
            }
        }
        return;
    }
 
    ///alpha-beta pruned negamax algorithm with iterative deepening
    fn search_static_d(
        &mut self,
        target_d: usize,
        idx: usize,
        move_gen: &MoveGen,
        zobrist: &Zobrist,
    ) {
        //cv: current variation, pv: primary variation
        fn inner(
            d: usize,
            target_d: usize,
            alpha: i32,
            beta: i32,
            pv: &mut [u32],
            prev_pv: &[u32],
            pos: &mut Position,
            evaluator: &Evaluator,
            search_data: &mut SearchData,
            move_gen: &MoveGen,
            zobrist: &Zobrist,
        ) -> i32 {
            search_data.positions_searched += 1;
            let (s, e) = pos.search_move_bounds();
            if s == e {
                //mate or stalemate
                if pos.board.nof_checkers > 0 {
                    return -MATE_EVAL + d as i32; //sooner mate is better
                } else {
                    return 0; //stalemate
                }
            } else if search_data.in_three_fold(pos) || pos.board.is_fifty_move_draw() {
                return 0;
            } else if d == target_d {
                return evaluator.eval(pos.board.pieces, pos.board.turn, pos.is_late_game());
            }

            let mut eval: i32 = EVAL_INIT;
            let mut new_alpha: i32 = alpha;
            for i in s..e {
                let mut child_pv: [u32; MAX_SEARCH_DEPTH + 1] = [NULL_MOVE; MAX_SEARCH_DEPTH + 1];
                let mov: u32 =
                    partial_selection_sort(&mut pos.move_arr[i..e], prev_pv[d], pos.last_target);
                pos.make_move(mov, true, false, move_gen, zobrist);
                search_data.board_hash_history.push(pos.board.zhash);
                let mov_eval: i32 = -inner(
                    d + 1,
                    target_d,
                    -beta,
                    -new_alpha,
                    &mut child_pv,
                    prev_pv,
                    pos,
                    evaluator,
                    search_data,
                    move_gen,
                    zobrist,
                );
                search_data.board_hash_history.pop();
                pos.unmake_move(mov, zobrist);

                if mov_eval > eval {
                    eval = mov_eval;
                    //child's pv becomes this node's pv
                    pv[d + 1..=target_d].copy_from_slice(&child_pv[d + 1..=target_d]);
                    pv[d] = mov;
                }

                new_alpha = max(new_alpha, mov_eval);

                if new_alpha >= beta {
                    search_data.ab_cutoffs += 1;
                    return new_alpha;
                }
            }
            return eval;
        }
        //iterative deepening:
        let synced_pv_depth: usize = self.count_pv_moves(idx);
        let pos: &mut Position = &mut self.positions[idx];
        let search_data: &mut SearchData = &mut self.search_data[idx];
        let mut pv: [u32; MAX_SEARCH_DEPTH + 1] = search_data.pv;
        for i in (synced_pv_depth + 1)..=target_d {
            let prev_pv: [u32; MAX_SEARCH_DEPTH + 1] = pv;
            pv.fill(NULL_MOVE);
            let eval: i32 = inner(
                0,
                i,
                ALPHA_INIT,
                BETA_INIT,
                &mut pv,
                &prev_pv,
                pos,
                &self.evaluator,
                search_data,
                move_gen,
                zobrist,
            );
            search_data.cumul_positions_searched += search_data.positions_searched;
            if self.search_config.log_diagnostics {
                println!("at depth: {}, eval: {}", i, eval);
                search_data.log_performance();
            }
            search_data.reset_temp_performance_data();
        }

        let mut i = 0;
        while i < MAX_SEARCH_DEPTH {
            search_data.pv[i] = pv[i];
            if pv[i] == NULL_MOVE {
                break;
            }
            i += 1;
        }
        if self.search_config.log_diagnostics {
            println!(
                "got pv: {:?}",
                search_data.pv.map(|m| _move::to_string(m, true))
            );
        }

        return;
    }

    fn search_static_time(
        &mut self,
        t: u64, //milliseconds
        idx: usize,
        move_gen: &MoveGen,
        zobrist: &Zobrist,
        kill_switch: Option<Arc<AtomicBool>>
    ) {
        let start: Instant = Instant::now();
        //cv: current variation, pv: primary variation
        fn inner(
            d: usize,
            target_d: usize,
            start_t: Instant,
            target_t: u64,
            alpha: i32,
            beta: i32,
            pv: &mut [u32],
            prev_pv: &[u32],
            pos: &mut Position,
            evaluator: &Evaluator,
            search_data: &mut SearchData,
            move_gen: &MoveGen,
            zobrist: &Zobrist,
            kill_switch: Option<Arc<AtomicBool>>
        ) -> i32 {
            if search_data.positions_searched != 0
                && ((search_data.positions_searched % 8192 == 0
                     && start_t.elapsed().as_millis() as u64 > target_t
                    ) || (
                     kill_switch.is_some() 
                     && kill_switch.as_ref().unwrap().load(Relaxed))
                    )
            {
                return EVAL_QUIT;
            }
            search_data.positions_searched += 1;
            let (s, e) = pos.search_move_bounds();
            if s == e {
                //mate or stalemate
                if pos.board.nof_checkers > 0 {
                    return -MATE_EVAL + d as i32; //sooner mate is better
                } else {
                    return 0; //stalemate
                }
            } else if search_data.in_three_fold(pos) || pos.board.is_fifty_move_draw() {
                return 0;
            } else if d == target_d {
                return evaluator.eval(pos.board.pieces, pos.board.turn, pos.is_late_game());
            }

            let mut eval: i32 = EVAL_INIT;
            let mut new_alpha: i32 = alpha;
            for i in s..e {
                let mut child_pv: [u32; MAX_SEARCH_DEPTH + 1] = [NULL_MOVE; MAX_SEARCH_DEPTH + 1];
                let mov: u32 =
                    partial_selection_sort(&mut pos.move_arr[i..e], prev_pv[d], pos.last_target);
                pos.make_move(mov, true, false, move_gen, zobrist);
                search_data.board_hash_history.push(pos.board.zhash);
                let child_eval: i32 = inner(
                    d + 1,
                    target_d,
                    start_t,
                    target_t,
                    -beta,
                    -new_alpha,
                    &mut child_pv,
                    prev_pv,
                    pos,
                    evaluator,
                    search_data,
                    move_gen,
                    zobrist,
                    kill_switch.clone()
                );
                search_data.board_hash_history.pop();
                pos.unmake_move(mov, zobrist);

                if child_eval == EVAL_QUIT {
                    return EVAL_QUIT;
                }

                let new_eval: i32 = -child_eval; //candidate for this node
                if new_eval > eval {
                    //child's pv becomes this node's pv
                    eval = new_eval;
                    pv[d + 1..=target_d].copy_from_slice(&child_pv[d + 1..=target_d]);
                    pv[d] = mov;
                }

                new_alpha = max(new_alpha, new_eval);

                if new_alpha >= beta {
                    search_data.ab_cutoffs += 1;
                    return new_alpha;
                }
            }
            return eval;
        }
        //iterative deepening:
        let synced_pv_depth: usize = self.count_pv_moves(idx);
        let pos: &mut Position = &mut self.positions[idx];
        let search_data: &mut SearchData = &mut self.search_data[idx];
        let mut pv: [u32; MAX_SEARCH_DEPTH + 1] = search_data.pv;
        for i in (synced_pv_depth + 1)..=MAX_SEARCH_DEPTH {
            let prev_pv: [u32; MAX_SEARCH_DEPTH + 1] = pv;
            pv.fill(NULL_MOVE);
            let eval: i32 = inner(
                0,
                i,
                start,
                t,
                ALPHA_INIT,
                BETA_INIT,
                &mut pv,
                &prev_pv,
                pos,
                &self.evaluator,
                search_data,
                move_gen,
                zobrist,
                kill_switch.clone()
            );
            search_data.cumul_positions_searched += search_data.positions_searched;
            if eval == EVAL_QUIT {
                search_data.reset_temp_performance_data();
                pv = prev_pv;
                break;
            }
            if self.search_config.log_diagnostics {
                println!("at depth: {}, eval: {}", i, eval);
                search_data.log_performance();
            }
            search_data.reset_temp_performance_data();
        }

        search_data.pv = pv;
        if self.search_config.log_diagnostics {
            println!(
                "got pv: {:?}",
                search_data.pv.map(|m| _move::to_string(m, true))
            );
        }

        return;
    }

    pub fn collect_best_move(&self) -> Option<u32> {
        if self.multithreaded {
            panic!("multithreaded search");
        } else {
            match self.search_data[0].pv[0] {
                NULL_MOVE => None,
                m => Some(m),
            }
        }
    }

    pub fn collect_ponder_move(&self) -> Option<u32> {
        if self.multithreaded {
            panic!("multithreaded search");
        } else {
            match self.search_data[0].pv[1] {
                NULL_MOVE => None,
                m => Some(m),
            }
        }
    }

    fn drop_pv_head(&mut self, idx: usize) {
        let mut new_pv: [u32; MAX_SEARCH_DEPTH + 1] = [NULL_MOVE; MAX_SEARCH_DEPTH + 1];
        new_pv[0..MAX_SEARCH_DEPTH]
            .copy_from_slice(&self.search_data[idx].pv[1..=MAX_SEARCH_DEPTH]);
        self.search_data[idx].pv = new_pv;
    }

    fn count_pv_moves(&self, idx: usize) -> usize {
        let mut i: usize = 0;
        while i < MAX_SEARCH_DEPTH {
            if self.search_data[idx].pv[i] == NULL_MOVE {
                break;
            }
            i += 1;
        }
        return i;
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
        let mov: u32 = move_arr_s[i];
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
            cur_v += EATING_MULTIPLIER
                * ((_move::eaten_piece(mov).unwrap() % NOF_PIECE_TYPES) as i32
                    - (_move::get_moved_piece(mov) % NOF_PIECE_TYPES) as i32);
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

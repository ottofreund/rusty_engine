use std::{cmp::max, sync::{Arc, atomic::{AtomicBool, Ordering::Relaxed}}, time::Instant};

use crate::{
    repr::{
        _move::{self, *}, board::Board, move_gen::MoveGen, position::Position,
    }, search::{
        eval::{Evaluator, MATE_EVAL, PIECE_MATERIAL_VALUE}, search_config::*, search_data::{SearchData, get_triang_pv_ply_idx_table}, tt::TranspositionTable,
    }, utils::zobrist::Zobrist,
};

pub const MAX_SEARCH_DEPTH: usize = 50;
const THREAD_COUNT: usize = 4;
const ALPHA_INIT: i32 = -1_000_000_000;
const BETA_INIT: i32 = 1_000_000_000;
const EVAL_INIT: i32 = -1_000_000_000;
const EVAL_QUIT: i32 = 555_555_555;

const PROMOTION_SCORE: i32 = 1_000;
const EATING_MULTIPLIER: i32 = 7;
const NON_CAPTURE_BONUS: i32 = 10_000;
const GOOD_CAPTURE_BONUS: i32 = 100_000;

pub struct Searcher {
    pub positions: Vec<Position>,
    pub search_data: Vec<SearchData>,
    pub multithreaded: bool,
    pub search_config: SearchConfig,
    pub evaluator: Evaluator,
    pub tt: TranspositionTable,
    last_sync_deviates_from_pv: bool,
}

//minimax with alpha beta pruning, ran by iterative deepening
//search heuristics in ordering of moves
impl Searcher {
    pub fn import_position(&mut self, pos: &Position, board_hash_history: Option<Vec<u64>>) {
        if self.multithreaded {
            for i in 0..THREAD_COUNT {
                self.positions[i] = (*pos).clone();
                self.search_data[i] = if let Some(bhh) = &board_hash_history {
                    SearchData::with_board_hash_history(pos, bhh.clone())
                } else {
                    SearchData::new(pos)
                };
            }
        } else {
            self.positions[0] = (*pos).clone();
            if let Some(bhh) = &board_hash_history {
                self.search_data[0] = SearchData::with_board_hash_history(pos, bhh.clone());
            } else {
                self.search_data[0] = SearchData::new(pos);
            }
        }
        
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
        let e: usize = if self.multithreaded { THREAD_COUNT } else { 1 };
        for i in 0..e {
            self.positions[i] = (*new_pos).clone();
            if mov.is_some() && _move::is_unrepeatable(mov.unwrap()) {
                self.search_data[i].board_hash_history.clear();
            }
            self.search_data[i]
                .board_hash_history
                .push(new_pos.board.zhash);

            if self.last_sync_deviates_from_pv || self.search_data[i].pv_ply_indices.len() < 2 {
                self.search_data[i].pv.fill(NULL_MOVE);
            } else {
                self.drop_pv_head(i);
            }
        }
    }

    pub fn from(pos: &Position, multithreaded: bool) -> Searcher {
        let positions: Vec<Position>;
        let search_data: Vec<SearchData>;
        if multithreaded {
            positions = (0..THREAD_COUNT).map(|_| (*pos).clone()).collect();
            search_data = (0..THREAD_COUNT).map(|_| SearchData::new(pos)).collect();
        } else {
            positions = vec![(*pos).clone()];
            search_data = vec![SearchData::new(pos)];
        }
        let search_config = SearchConfig::default();
        return Self {
            positions,
            search_data,
            multithreaded: multithreaded,
            search_config,
            evaluator: Evaluator::default(),
            tt: TranspositionTable::default(),
            last_sync_deviates_from_pv: true,
        };
    }

    pub fn start_search(&mut self, move_gen: &MoveGen, zobrist: &Zobrist, kill_switch: Option<Arc<AtomicBool>>) {
        self.tt.generation = self.tt.generation.wrapping_add(1);
        if self.multithreaded {
            panic!("multithreaded search");
            //PRAGMA FOR LOOP HERE
            /* for i in 0..THREAD_COUNT {
                self.start_search_node(i, move_gen, zobrist, kill_switch.clone());
            } */
        } else {
            self.start_search_node(0, move_gen, zobrist, kill_switch.as_deref());
        }
    }

    fn start_search_node(&mut self, idx: usize, move_gen: &MoveGen, zobrist: &Zobrist, kill_switch: Option<&AtomicBool>) {
        self.search_data[idx].age_history();
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
        assert!(
            target_d <= MAX_SEARCH_DEPTH,
            "static search depth {target_d} exceeds MAX_SEARCH_DEPTH {MAX_SEARCH_DEPTH}"
        );

        fn inner(
            d: usize,
            target_d: usize,
            mut alpha: i32,
            beta: i32,
            mut in_quiescence: bool,
            use_quiescence: bool,
            mut follows_prev_pv: bool,
            prev_pv: &[u32],
            pos: &mut Position,
            evaluator: &Evaluator,
            search_data: &mut SearchData,
            move_gen: &MoveGen,
            zobrist: &Zobrist,
        ) -> i32 {
            if d < target_d {
                let row_start = search_data.pv_ply_indices[d];
                search_data.pv[row_start] = NULL_MOVE;
            }

            search_data.positions_searched += 1;
            search_data.sel_depth = max(search_data.sel_depth, d);
            let mut eval: i32 = EVAL_INIT;
            let (s, e) = pos.search_move_bounds();
            if s == e {
                if pos.board.nof_checkers > 0 {
                    return -MATE_EVAL + d as i32; //sooner mate is better
                } else if in_quiescence {
                    return evaluator.eval(pos.board.pieces, pos.board.turn, pos.is_late_game()); //might miss stalemate but not worth it to check for performance reasons
                } else {
                    return 0; //stalemate
                }
            } else if search_data.in_three_fold(pos) || pos.board.is_fifty_move_draw() {
                return 0;
            } else if d >= target_d {
                if use_quiescence {
                    if pos.board.nof_checkers == 0 {
                        eval = evaluator.eval(pos.board.pieces, pos.board.turn, pos.is_late_game());
                        if eval >= beta {
                            search_data.stand_pat_cutoffs += 1;
                            return eval;
                        }
                        alpha = max(alpha, eval);
                    }
                } else {
                    return evaluator.eval(pos.board.pieces, pos.board.turn, pos.is_late_game());
                }
            }

            if d == target_d - 1 && use_quiescence {
                in_quiescence = true;
            }

            let mut only_bad_captures_left: Option<bool> = None;

            for i in s..e {
                let pv_mv: u32 = if follows_prev_pv && d < prev_pv.len() && i == s { // i == s because pv always gets picked first after which not available
                    prev_pv[d]
                } else {
                    NULL_MOVE
                };
                let mov: u32 =
                    Searcher::partial_selection_sort(&mut pos.move_arr[i..e], pv_mv, &mut only_bad_captures_left, move_gen, search_data, &pos.board);

                follows_prev_pv = follows_prev_pv && mov == pv_mv;

                pos.make_move(mov, true, false, in_quiescence, move_gen, zobrist);
                search_data.board_hash_history.push(pos.board.zhash);
                let mov_eval: i32 = -inner(
                    d + 1,
                    target_d,
                    -beta,
                    -alpha,
                    in_quiescence,
                    use_quiescence,
                    follows_prev_pv,
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
                    if d < target_d {
                        let cur_ply_s_idx: usize = search_data.pv_ply_indices[d];
                        let child_ply_s_idx: usize = cur_ply_s_idx + (target_d - d);
                        let child_ply_e_idx: usize =
                            child_ply_s_idx + (target_d - (d + 1));
                        search_data.pv.copy_within(
                            child_ply_s_idx..child_ply_e_idx,
                            cur_ply_s_idx + 1,
                        );
                        search_data.pv[cur_ply_s_idx] = mov;
                    }
                }

                alpha = max(alpha, mov_eval);

                if alpha >= beta {
                    search_data.ab_cutoffs += 1;
                    if d < target_d {
                        Searcher::update_quiet_history_after_cutoff(
                            search_data,
                            pos.board.turn,
                            mov,
                            &pos.move_arr[s..i],
                            target_d - d,
                        );
                    }
                    return alpha;
                }
            }
            return eval;
        }
        //iterative deepening:
        let synced_pv_depth: usize = self.count_pv_moves(idx);
        let mut completed_pv_len: usize = synced_pv_depth;
        let pos: &mut Position = &mut self.positions[idx];
        let search_data: &mut SearchData = &mut self.search_data[idx];
        for d in (synced_pv_depth + 1)..=target_d {
            let mut prev_pv = vec![NULL_MOVE; d];
            prev_pv[..completed_pv_len]
                .copy_from_slice(&search_data.pv[..completed_pv_len]);
            search_data.pv_ply_indices = get_triang_pv_ply_idx_table(d);
            let eval: i32 = inner(
                0,
                d,
                ALPHA_INIT,
                BETA_INIT,
                false,
                self.search_config.quiescence,
                true,
                &prev_pv,
                pos,
                &self.evaluator,
                search_data,
                move_gen,
                zobrist,
            );
            search_data.cumul_positions_searched += search_data.positions_searched;

            completed_pv_len = search_data.pv[..d]
                .iter()
                .position(|mov| *mov == NULL_MOVE)
                .unwrap_or(d);

            if self.search_config.log_uci_diagnostics {
                println!(
                    "info depth {d} seldepth {} score cp {eval} nodes {} ab-cutoffs {} stand-pat-cutoffs {} pv {}", 
                    search_data.sel_depth, search_data.positions_searched, search_data.ab_cutoffs, search_data.stand_pat_cutoffs, search_data.pv[0..completed_pv_len].iter().map(|m| _move::to_string(*m, true)).collect::<Vec<String>>().join(" ")
                );
            }
            search_data.reset_temp_performance_data();
        }

        return;
    }

    fn search_static_time(
        &mut self,
        t: u64, //milliseconds
        idx: usize,
        move_gen: &MoveGen,
        zobrist: &Zobrist,
        kill_switch: Option<&AtomicBool>
    ) {
        let start: Instant = Instant::now();
        fn inner(
            d: usize,
            target_d: usize,
            start_t: Instant,
            target_t: u64,
            mut alpha: i32,
            beta: i32,
            mut in_quiescence: bool,
            use_quiescence: bool,
            mut follows_prev_pv: bool,
            prev_pv: &[u32],
            pos: &mut Position,
            evaluator: &Evaluator,
            search_data: &mut SearchData,
            move_gen: &MoveGen,
            zobrist: &Zobrist,
            kill_switch: Option<&AtomicBool>
        ) -> i32 {
            if search_data.positions_searched != 0
                && ((search_data.positions_searched % 8192 == 0
                     && start_t.elapsed().as_millis() as u64 > target_t
                    ) || (
                     kill_switch.is_some() 
                     && kill_switch.unwrap().load(Relaxed))
                    )
            {
                return EVAL_QUIT;
            }

            if d < target_d {
                let row_start = search_data.pv_ply_indices[d];
                search_data.pv[row_start] = NULL_MOVE;
            }

            search_data.positions_searched += 1;
            search_data.sel_depth = max(search_data.sel_depth, d);

            let mut eval: i32 = EVAL_INIT;
            let (s, e) = pos.search_move_bounds();
            if s == e {
                if pos.board.nof_checkers > 0 {
                    return -MATE_EVAL + d as i32; //sooner mate is better
                } else if in_quiescence {
                    return evaluator.eval(pos.board.pieces, pos.board.turn, pos.is_late_game()); //might miss stalemate but not worth it to check for performance reasons
                } else {
                    return 0; //stalemate
                }
            } else if search_data.in_three_fold(pos) || pos.board.is_fifty_move_draw() {
                return 0;
            } else if d >= target_d {
                if use_quiescence {
                    if pos.board.nof_checkers == 0 {
                        eval = evaluator.eval(pos.board.pieces, pos.board.turn, pos.is_late_game());
                        if eval >= beta {
                            search_data.stand_pat_cutoffs += 1;
                            return eval;
                        }
                        alpha = max(alpha, eval);
                    }
                } else {
                    return evaluator.eval(pos.board.pieces, pos.board.turn, pos.is_late_game());
                }
            }

            if d == target_d - 1 && use_quiescence {
                in_quiescence = true;
            }

            let mut only_bad_captures_left: Option<bool> = None;

            for i in s..e {
                let prev_pv_mv: u32 = if follows_prev_pv && d < prev_pv.len() && i == s { prev_pv[d] } else { NULL_MOVE }; // i == s because pv always gets picked first after which not available
                let mov: u32 =
                    Searcher::partial_selection_sort(&mut pos.move_arr[i..e], prev_pv_mv, &mut only_bad_captures_left, move_gen, search_data, &pos.board);

                follows_prev_pv = follows_prev_pv && mov == prev_pv_mv;

                pos.make_move(mov, true, false, in_quiescence, move_gen, zobrist);
                search_data.board_hash_history.push(pos.board.zhash);
                let child_eval: i32 = inner(
                    d + 1,
                    target_d,
                    start_t,
                    target_t,
                    -beta,
                    -alpha,
                    in_quiescence,
                    use_quiescence,
                    follows_prev_pv,
                    prev_pv,
                    pos,
                    evaluator,
                    search_data,
                    move_gen,
                    zobrist,
                    kill_switch
                );
                search_data.board_hash_history.pop();
                pos.unmake_move(mov, zobrist);

                if child_eval == EVAL_QUIT {
                    return EVAL_QUIT;
                }

                let new_eval: i32 = -child_eval; //candidate for this node
                if new_eval > eval {
                    //child ply's pv appended to this ply's pv
                    eval = new_eval;
                    if d < target_d {
                        let cur_ply_s_idx: usize = search_data.pv_ply_indices[d];
                        let child_ply_s_idx: usize = cur_ply_s_idx + (target_d - d);
                        let child_ply_e_idx: usize = child_ply_s_idx + (target_d - (d + 1));
                        search_data.pv.copy_within(child_ply_s_idx..child_ply_e_idx, cur_ply_s_idx + 1);
                        search_data.pv[cur_ply_s_idx] = mov;
                    }
                }

                alpha = max(alpha, new_eval);

                if alpha >= beta {
                    search_data.ab_cutoffs += 1;
                    if d < target_d {
                        Searcher::update_quiet_history_after_cutoff(
                            search_data,
                            pos.board.turn,
                            mov,
                            &pos.move_arr[s..i],
                            target_d - d,
                        );
                    }
                    return alpha;
                }
            }
            return eval;
        }
        //iterative deepening:
        let synced_pv_depth: usize = self.count_pv_moves(idx);
        let mut completed_pv_len: usize = synced_pv_depth;
        let pos: &mut Position = &mut self.positions[idx];
        let search_data: &mut SearchData = &mut self.search_data[idx];
        for d in (synced_pv_depth + 1)..=MAX_SEARCH_DEPTH {
            let mut prev_pv = vec![NULL_MOVE; d];
            prev_pv[..completed_pv_len]
                .copy_from_slice(&search_data.pv[..completed_pv_len]);
            search_data.pv_ply_indices = get_triang_pv_ply_idx_table(d);
            let eval: i32 = inner(
                0,
                d,
                start,
                t,
                ALPHA_INIT,
                BETA_INIT,
                false,
                self.search_config.quiescence,
                true,
                &prev_pv,
                pos,
                &self.evaluator,
                search_data,
                move_gen,
                zobrist,
                kill_switch
            );
            search_data.cumul_positions_searched += search_data.positions_searched;
            if eval == EVAL_QUIT {
                search_data.reset_temp_performance_data();
                if search_data.pv[0] == NULL_MOVE { //didn't finish any root move in time
                    search_data.pv[..d].copy_from_slice(&prev_pv);
                }
                break;
            }

            completed_pv_len = search_data.pv[..d]
                .iter()
                .position(|mov| *mov == NULL_MOVE)
                .unwrap_or(d);

            if self.search_config.log_uci_diagnostics {
                println!(
                    "info depth {d} seldepth {} score cp {eval} nodes {} ab-cutoffs {} stand-pat-cutoffs {} pv {}", 
                    search_data.sel_depth, search_data.positions_searched, search_data.ab_cutoffs, search_data.stand_pat_cutoffs, search_data.pv[0..completed_pv_len].iter().map(|m| _move::to_string(*m, true)).collect::<Vec<String>>().join(" ")
                );
            }
            search_data.reset_temp_performance_data();
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
            if self.count_pv_moves(0) > 1 {
                match self.search_data[0].pv[1] {
                    NULL_MOVE => None,
                    m => Some(m),
                }
            } else {
                return None;
            }
            
        }
    }

    fn drop_pv_head(&mut self, idx: usize) {
        let head_ply_e_idx: usize = self.search_data[idx].pv_ply_indices[1];
        self.search_data[idx].pv.copy_within(1..head_ply_e_idx, 0);
        self.search_data[idx].pv[head_ply_e_idx - 1] = NULL_MOVE;
    }

    fn count_pv_moves(&self, idx: usize) -> usize {
        let mut i: usize = 0;
        let root_pv_e_idx: usize = self.search_data[idx].pv_ply_indices[1];
        while i < root_pv_e_idx {
            if self.search_data[idx].pv[i] == NULL_MOVE {
                break;
            }
            i += 1;
        }
        return i;
    }

    #[inline]
    fn update_quiet_history_after_cutoff(
        search_data: &mut SearchData,
        side: u32,
        cutoff_move: u32,
        previously_searched_moves: &[u32],
        remaining_depth: usize,
    ) {
        if _move::is_eating(cutoff_move) || _move::is_promotion(cutoff_move) {
            return;
        }

        let bonus = remaining_depth as i32;
        search_data.update_history_entry(
            side,
            _move::get_init(cutoff_move),
            _move::get_target(cutoff_move),
            bonus,
        );

        for &previous_move in previously_searched_moves {
            if !_move::is_eating(previous_move) && !_move::is_promotion(previous_move) {
                search_data.update_history_entry(
                    side,
                    _move::get_init(previous_move),
                    _move::get_target(previous_move),
                    -bonus,
                );
            }
        }
    }

    //k == 1, so "selection pick", in place
    //only_bad_captures_left is on all calls after first bad capture is returned
    fn partial_selection_sort(
        move_arr_s: &mut [u32],
        pv_mv: u32,
        only_bad_captures_left: &mut Option<bool>,
        move_gen: &MoveGen,
        search_data: &mut SearchData,
        board: &Board
    ) -> u32 {
        let mut best_i: usize = usize::MAX;
        if pv_mv != NULL_MOVE {
            let idx = move_arr_s.iter().position(|m| *m == pv_mv).expect("pv move was some but not generated");
            best_i = idx;
        } else { //no pv move available
            //non-dominating is non-captures if !*only_bad_captures_left, else if *only_bad_captures_left then only bad captures.
            //If only_bad_captures_left == None, then non-dominating are both non-captures and bad captures
            let mut best_v: i32 = i32::MIN;
            let mut found_dominating: bool = false;
            let mut non_capture_count: usize = 0;

            for i in 0..move_arr_s.len() {
                let mov: u32 = move_arr_s[i];
                let mut cur_v: i32 = 0;
                if _move::is_eating(mov) {
                    if _move::is_negative_see(mov) {
                        if let Some(only_bad_left) = only_bad_captures_left {
                            if *only_bad_left {
                                cur_v = EATING_MULTIPLIER * PIECE_MATERIAL_VALUE[_move::eaten_piece(mov).unwrap() as usize];
                            } //else quiets left so ignore bad capture
                        } else { //quiets may remain but we don't know yet
                            cur_v = EATING_MULTIPLIER * PIECE_MATERIAL_VALUE[_move::eaten_piece(mov).unwrap() as usize];
                        }
                    } else if _move::is_positive_see(mov) {
                        cur_v = GOOD_CAPTURE_BONUS + EATING_MULTIPLIER * PIECE_MATERIAL_VALUE[_move::eaten_piece(mov).unwrap() as usize];
                        found_dominating = true;
                    } else { //unevaluated capture
                        let is_good_capture: bool =
                            search_data.see_helper.see_positive(mov, board, move_gen);

                        if is_good_capture {
                            move_arr_s[i] = _move::with_positive_see(mov);
                            cur_v = GOOD_CAPTURE_BONUS + EATING_MULTIPLIER * PIECE_MATERIAL_VALUE[_move::eaten_piece(mov).unwrap() as usize];
                            found_dominating = true;
                        } else {
                            move_arr_s[i] = _move::with_negative_see(mov);
                            if let Some(only_bad_left) = only_bad_captures_left {
                                if *only_bad_left {
                                    cur_v = EATING_MULTIPLIER * PIECE_MATERIAL_VALUE[_move::eaten_piece(mov).unwrap() as usize];
                                } //else quiets left so ignore bad capture
                            } else { //quiets may remain but we don't know yet
                                cur_v = EATING_MULTIPLIER * PIECE_MATERIAL_VALUE[_move::eaten_piece(mov).unwrap() as usize];
                            }
                        }
                    }
                    if _move::is_promotion(mov) {
                        cur_v += PROMOTION_SCORE + _move::get_promotion_piece(mov) as i32;
                    }
                } else if !found_dominating { //non-capture still candidate
                    if _move::is_promotion(mov) {
                        cur_v += PROMOTION_SCORE + _move::get_promotion_piece(mov) as i32;
                    } else {
                        cur_v += search_data.get_history_entry(board.turn, mov)
                    }
                    cur_v += NON_CAPTURE_BONUS; //give edge over bad captures
                    non_capture_count += 1;
                } else {
                    non_capture_count += 1;
                }

                if cur_v > best_v {
                    best_v = cur_v;
                    best_i = i;
                }
            }

            if non_capture_count == 0 {
                *only_bad_captures_left = Some(true);
            }
        }

        move_arr_s.swap(0, best_i);
        move_arr_s[0] = _move::with_see_cleared(move_arr_s[0]);
        return move_arr_s[0];
    }

}

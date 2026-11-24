use std::{cmp::{max, min}, sync::{Arc, atomic::{AtomicBool, Ordering::Relaxed}}, time::Instant};

use crate::{
    repr::{
        _move::{self, *}, board::Board, move_gen::MoveGen, position::Position,
    }, search::{
        eval::{Evaluator, MATE_BOUND, MATE_EVAL, PIECE_MATERIAL_VALUE}, search_config::*, search_data::{SearchData, get_triang_pv_ply_idx_table}, tt::{TTEntry, TTEntryType, TranspositionTable},
    }, utils::zobrist::Zobrist,
};

pub const MAX_SEARCH_DEPTH: usize = 50;
const THREAD_COUNT: usize = 4;
const STOP_CHECK_INTERVAL: u64 = 8192;
const ALPHA_INIT: i16 = -i16::MAX;
const BETA_INIT: i16 = i16::MAX;
const EVAL_INIT: i16 = -i16::MAX;
const EVAL_QUIT: i16 = 31111;
const FUTILITY_MARGIN: i16 = 60;

const PROMOTION_SCORE: i32 = 1_000;
const EATING_MULTIPLIER: i32 = 7;
const NON_CAPTURE_BONUS: i32 = 10_000;
const GOOD_CAPTURE_BONUS: i32 = 100_000;

struct SearchControl<'a> {
    time_limit: Option<(Instant, u64)>,
    kill_switch: Option<&'a AtomicBool>,
}

impl<'a> SearchControl<'a> {
    fn new(target_time: Option<u64>, kill_switch: Option<&'a AtomicBool>) -> Self {
        Self {
            time_limit: target_time.map(|target_time| (Instant::now(), target_time)),
            kill_switch,
        }
    }

    fn should_stop(&self, positions_searched: u64) -> bool {
        positions_searched != 0
            && positions_searched % STOP_CHECK_INTERVAL == 0
            && (self
                .time_limit
                .as_ref()
                .is_some_and(|(start, target_time)| {
                    start.elapsed().as_millis() as u64 > *target_time
                })
                || self
                    .kill_switch
                    .is_some_and(|kill_switch| kill_switch.load(Relaxed)))
    }
}

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

    /// Between games reset
    pub fn reset(&mut self) {
        self.tt.clear();
        if self.multithreaded {
            for i in 0..THREAD_COUNT {
                self.search_data[i].reset();
            }
        } else {
            self.search_data[0].reset();
        }
        self.last_sync_deviates_from_pv = true;
    }

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
                .push(new_pos.zhash);

            if self.last_sync_deviates_from_pv || self.search_data[i].pv_ply_indices.len() < 2 {
                self.search_data[i].pv.fill(NULL_MOVE);
                self.search_data[i].completed_depth = 0;
            } else {
                self.drop_pv_head(i);
                let search_data = &mut self.search_data[i];
                // A truncated PV can leave the new root without a move.
                search_data.completed_depth = if search_data.pv[0] == NULL_MOVE {
                    0
                } else {
                    search_data.completed_depth.saturating_sub(1)
                };
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
        let (target_depth, target_time) = match self.search_config.search_mode {
            SearchMode::StaticDepth(d) => {
                assert!(
                    d <= MAX_SEARCH_DEPTH,
                    "static search depth {d} exceeds MAX_SEARCH_DEPTH {MAX_SEARCH_DEPTH}"
                );
                (d, None)
            }
            SearchMode::StaticTime(t) => (MAX_SEARCH_DEPTH, Some(t)),
        };
        self.search(
            target_depth,
            target_time,
            idx,
            move_gen,
            zobrist,
            kill_switch,
        );
    }
 
    ///alpha-beta pruned negamax algorithm with iterative deepening
    fn search(
        &mut self,
        target_depth: usize,
        target_time: Option<u64>, //milliseconds
        idx: usize,
        move_gen: &MoveGen,
        zobrist: &Zobrist,
        kill_switch: Option<&AtomicBool>,
    ) {
        let control = SearchControl::new(target_time, kill_switch);

        fn inner(
            d: i16,
            ply: usize,
            target_d: usize,
            mut alpha: i16,
            mut beta: i16,
            use_quiescence: bool,
            follows_prev_pv: bool,
            is_pv_node: bool,
            is_null_window: bool,
            prev_pv: &[u32],
            pos: &mut Position,
            evaluator: &Evaluator,
            search_data: &mut SearchData,
            move_gen: &MoveGen,
            zobrist: &Zobrist,
            control: &SearchControl<'_>,
            tt: &mut TranspositionTable,
        ) -> i16 {
            if d <= 0 && use_quiescence {
                return Searcher::quiescence(
                    ply, alpha, beta, is_pv_node, pos, evaluator, search_data, move_gen, zobrist,
                    control, tt,
                );
            }

            if control.should_stop(search_data.positions_searched) {
                return EVAL_QUIT;
            }

            if d > 0 { //initialize triangular pv row for this ply
                let row_start = search_data.pv_ply_indices[ply];
                search_data.pv[row_start] = NULL_MOVE;
            }

            search_data.positions_searched += 1;
            search_data.sel_depth = max(search_data.sel_depth, ply);

            let (s, e) = pos.search_move_bounds();
            let is_three_fold: bool = search_data.in_three_fold(pos);
            //terminal conditions
            if s == e {
                if pos.board.nof_checkers > 0 {
                    return -MATE_EVAL + ply as i16;
                }
                return 0; // stalemate
            }
            if is_three_fold || pos.board.is_fifty_move_draw() || (pos.board.has_insufficient_material() && ply != 0) {
                return 0;
            }
            if d <= 0 {
                return evaluator.eval(pos.board.pieces, pos.board.turn, pos.board.late_game_phase);
            }

            let depth = d.max(0) as u8;
            let key = Zobrist::zkey50_adjusted(pos.zhash, pos.board.half_move_clock);
            let tte = Searcher::probe_tt(tt, key, &pos.move_arr[s..e], ply);
            if let Some(tt_entry) = tte {
                if !is_pv_node
                    && !is_three_fold
                    && pos.board.half_move_clock < 96
                    && tt_entry.depth() >= depth
                {
                    match tt_entry.bound_type() {
                        TTEntryType::Exact => {
                            if d > 0 {
                                let row_start = search_data.pv_ply_indices[ply];
                                let row_end = search_data.pv_ply_indices[ply + 1];
                                search_data.pv[row_start..row_end].fill(NULL_MOVE);
                                search_data.pv[row_start] = tt_entry.best_move;
                            }
                            return tt_entry.score;
                        }
                        TTEntryType::LowerBound => alpha = max(alpha, tt_entry.score),
                        TTEntryType::UpperBound => beta = min(beta, tt_entry.score),
                    }
                    if alpha >= beta {
                        return tt_entry.score;
                    }
                }
            }

            let futility_pruning = d == 1
                && ply > 0
                && pos.board.nof_checkers == 0
                && alpha >= 0
                && beta < MATE_BOUND
                && evaluator.eval(pos.board.pieces, pos.board.turn, pos.board.late_game_phase)
                    <= alpha - FUTILITY_MARGIN;
            let mut check_info = None;
            let (old_alpha, old_beta) = (alpha, beta);
            let root_relative_d = target_d - ply.min(target_d); // LMR can change d
            let mut eval = EVAL_INIT;
            let mut best_move: u32 = NULL_MOVE;
            let mut only_bad_captures_left: Option<bool> = None;

            let prev_pv_mv: u32 = if follows_prev_pv && ply < prev_pv.len() { prev_pv[ply] } else { NULL_MOVE };
            let (mut primary_selection, mut secondary_selection) = match tte {
                Some(entry) if prev_pv_mv == NULL_MOVE || entry.depth() >= depth => {
                    (entry.best_move, prev_pv_mv)
                }
                Some(entry) => (prev_pv_mv, entry.best_move),
                None => (prev_pv_mv, NULL_MOVE),
            };
            if primary_selection == secondary_selection {
                secondary_selection = NULL_MOVE;
            }

            let mut i = s;
            let mut end = e;
            while i < end {
                let mov: u32 =
                    Searcher::partial_selection_sort(&mut pos.move_arr[i..end], primary_selection, secondary_selection, &mut only_bad_captures_left, move_gen, search_data, &pos.board);

                // EVAL_INIT also keeps the first move in the search.
                if futility_pruning
                    && eval > -MATE_BOUND
                    && mov != primary_selection
                    && mov != secondary_selection
                    && !is_eating(mov)
                    && !is_promotion(mov)
                    && !is_castle(mov)
                {
                    let info = check_info.get_or_insert_with(|| move_gen.check_info(&pos.board));
                    if !move_gen.gives_check(&pos.board, info, mov) {
                        // Keep the searched prefix intact for history updates.
                        end -= 1;
                        pos.move_arr.swap(i, end);
                        continue;
                    }
                }

                if mov == primary_selection {
                    primary_selection = NULL_MOVE;
                } else if mov == secondary_selection {
                    secondary_selection = NULL_MOVE;
                }

                let child_follows_prev_pv = follows_prev_pv && mov == prev_pv_mv;
                
                pos.make_move(mov, true, false, use_quiescence && d == 1, move_gen, zobrist);
                search_data.board_hash_history.push(pos.zhash);

                let lmr: i16 = if d < 3 || i == s || follows_prev_pv { 
                    0
                } else {
                    (0.99 + f32::ln(d as f32) * f32::ln((i - s + 1) as f32) / 3.14) as i16 //from Obsidian
                }; 
                let mut new_eval: i16 = EVAL_INIT;
                if i == s && is_pv_node { //full-window search
                    new_eval = -inner(
                        d - 1,
                        ply + 1,
                        target_d,
                        -beta,
                        -alpha,
                        use_quiescence,
                        child_follows_prev_pv,
                        true,
                        false,
                        prev_pv,
                        pos,
                        evaluator,
                        search_data,
                        move_gen,
                        zobrist,
                        control,
                        tt
                    );
                } else { //LMR null-window search ---(raises alpha)--> null-window search ---(raises alpha, doesn't fail-high)--> full-window search
                    if lmr > 0 {
                        new_eval = -inner(
                            d - 1 - lmr,
                            ply + 1,
                            target_d,
                            -alpha - 1,
                            -alpha,
                            use_quiescence,
                            child_follows_prev_pv,
                            false,
                            true,
                            prev_pv,
                            pos,
                            evaluator,
                            search_data,
                            move_gen,
                            zobrist,
                            control,
                            tt
                        );
                    }
                    
                    if new_eval.abs() != EVAL_QUIT && (lmr == 0 || new_eval > alpha) { //null-window search
                        new_eval = -inner(
                            d - 1,
                            ply + 1,
                            target_d,
                            -alpha - 1,
                            -alpha,
                            use_quiescence,
                            child_follows_prev_pv,
                            false,
                            true,
                            prev_pv,
                            pos,
                            evaluator,
                            search_data,
                            move_gen,
                            zobrist,
                            control,
                            tt
                        );
                        
                        if new_eval.abs() != EVAL_QUIT && new_eval > alpha && new_eval < beta && is_pv_node { //re-search with full window
                            new_eval = -inner(
                                d - 1,
                                ply + 1,
                                target_d,
                                -beta,
                                -alpha,
                                use_quiescence,
                                child_follows_prev_pv,
                                true,
                                false,
                                prev_pv,
                                pos,
                                evaluator,
                                search_data,
                                move_gen,
                                zobrist,
                                control,
                                tt
                            );
                        } //else if new_eval > alpha : let be treated like normal fail-high cutoff
                    }
                }

                search_data.board_hash_history.pop();
                pos.unmake_move(mov, zobrist);

                if new_eval.abs() == EVAL_QUIT {
                    return EVAL_QUIT;
                }

                if new_eval > eval {
                    eval = new_eval;
                    best_move = mov;
                    
                    if !is_null_window { //child ply's pv appended to this ply's pv
                        let cur_ply_s_idx: usize = search_data.pv_ply_indices[ply];
                        let child_ply_s_idx: usize = cur_ply_s_idx + root_relative_d;
                        let child_ply_e_idx: usize = child_ply_s_idx + root_relative_d - 1;
                        search_data.pv.copy_within(child_ply_s_idx..child_ply_e_idx, cur_ply_s_idx + 1);
                        search_data.pv[cur_ply_s_idx] = mov;
                    }
                }

                alpha = max(alpha, new_eval);

                if alpha >= beta {
                    search_data.ab_cutoffs += 1;
                    Searcher::update_quiet_history_after_cutoff(
                        search_data,
                        pos.board.turn,
                        mov,
                        &pos.move_arr[s..i],
                        d as usize,
                    );
                    break;
                }
                i += 1;
            }
            if end != e {
                // Treat skipped moves as failing low against the original alpha.
                eval = max(eval, old_alpha);
            }
            Searcher::store_tt(tt, key, best_move, depth, eval, ply, old_alpha, old_beta);
            eval
        }
        //iterative deepening:
        let mut pv_len: usize = self.count_pv_moves(idx);
        let use_quiescence = self.search_config.quiescence;
        let log_uci_diagnostics = self.search_config.log_uci_diagnostics;
        let pos: &mut Position = &mut self.positions[idx];
        let search_data: &mut SearchData = &mut self.search_data[idx];
        for td in (search_data.completed_depth + 1)..=target_depth {
            let mut prev_pv = vec![NULL_MOVE; td];
            let ordering_pv_len = pv_len.min(td);
            prev_pv[..ordering_pv_len]
                .copy_from_slice(&search_data.pv[..ordering_pv_len]);
            search_data.pv_ply_indices = get_triang_pv_ply_idx_table(td);
            let eval: i16 = inner(
                td as i16,
                0,
                td,
                ALPHA_INIT,
                BETA_INIT,
                use_quiescence,
                true,
                true,
                false,
                &prev_pv,
                pos,
                &self.evaluator,
                search_data,
                move_gen,
                zobrist,
                &control,
                &mut self.tt
            );
            search_data.cumul_positions_searched += search_data.positions_searched;
            if eval == EVAL_QUIT {
                search_data.reset_temp_performance_data();
                if search_data.pv[0] == NULL_MOVE { //didn't finish any root move before stopping
                    search_data.pv[..td].copy_from_slice(&prev_pv);
                }
                break;
            }

            search_data.completed_depth = td;
            pv_len = search_data.pv[..td]
                .iter()
                .position(|mov| *mov == NULL_MOVE)
                .unwrap_or(td);

            if log_uci_diagnostics {
                let (score_kind, score) = if eval.abs() >= MATE_BOUND {
                    let mate_plies = MATE_EVAL - eval.abs();
                    ("mate", ((mate_plies + 1) / 2) * eval.signum())
                } else {
                    ("cp", eval)
                };
                println!(
                    "info depth {td} seldepth {} score {score_kind} {score} nodes {} ab-cutoffs {} stand-pat-cutoffs {} pv {}",
                    search_data.sel_depth, search_data.positions_searched, search_data.ab_cutoffs, search_data.stand_pat_cutoffs, search_data.pv[0..pv_len].iter().map(|m| _move::to_string(*m, true)).collect::<Vec<String>>().join(" ")
                );
            }
            search_data.reset_temp_performance_data();
        }
    }

    fn quiescence(
        ply: usize,
        mut alpha: i16,
        mut beta: i16,
        is_pv_node: bool,
        pos: &mut Position,
        evaluator: &Evaluator,
        search_data: &mut SearchData,
        move_gen: &MoveGen,
        zobrist: &Zobrist,
        control: &SearchControl<'_>,
        tt: &mut TranspositionTable,
    ) -> i16 {
        if control.should_stop(search_data.positions_searched) {
            return EVAL_QUIT;
        }

        search_data.positions_searched += 1;
        search_data.sel_depth = max(search_data.sel_depth, ply);

        if pos.board.has_insufficient_material() {
            return 0;
        }

        let is_three_fold = search_data.in_three_fold(pos);
        // A reduced search can enter with a full move list.
        let (s, e) = pos.search_move_bounds();
        let key = Zobrist::zkey50_adjusted(pos.zhash, pos.board.half_move_clock);
        let tte = Self::probe_tt(tt, key, &pos.move_arr[s..e], ply);
        if let Some(tt_entry) = tte {
            if !is_pv_node && !is_three_fold && pos.board.half_move_clock < 96 {
                match tt_entry.bound_type() {
                    TTEntryType::Exact => return tt_entry.score,
                    TTEntryType::LowerBound => alpha = max(alpha, tt_entry.score),
                    TTEntryType::UpperBound => beta = min(beta, tt_entry.score),
                }
                if alpha >= beta {
                    return tt_entry.score;
                }
            }
        }

        let in_check = pos.board.nof_checkers > 0;
        if s == e && in_check {
            return -MATE_EVAL + ply as i16;
        }
        if is_three_fold || pos.board.is_fifty_move_draw() {
            return 0;
        }

        let (old_alpha, old_beta) = (alpha, beta);
        let mut eval = EVAL_INIT;
        if !in_check {
            eval = evaluator.eval(pos.board.pieces, pos.board.turn, pos.board.late_game_phase);
            if s == e {
                return eval;
            }
            if eval >= beta {
                search_data.stand_pat_cutoffs += 1;
                return eval;
            }
            alpha = max(alpha, eval);
        }

        let mut best_move = NULL_MOVE;
        let mut primary_selection = tte.map_or(NULL_MOVE, |entry| entry.best_move);
        let mut only_bad_captures_left = None;
        for i in s..e {
            let mov = Self::partial_selection_sort(
                &mut pos.move_arr[i..e],
                primary_selection,
                NULL_MOVE,
                &mut only_bad_captures_left,
                move_gen,
                search_data,
                &pos.board,
            );
            primary_selection = NULL_MOVE;

            pos.make_move(mov, true, false, true, move_gen, zobrist);
            search_data.board_hash_history.push(pos.zhash);

            let new_eval = -Self::quiescence(
                ply + 1,
                -beta,
                -alpha,
                is_pv_node,
                pos,
                evaluator,
                search_data,
                move_gen,
                zobrist,
                control,
                tt,
            );

            search_data.board_hash_history.pop();
            pos.unmake_move(mov, zobrist);

            if new_eval.abs() == EVAL_QUIT {
                return EVAL_QUIT;
            }
            if new_eval > eval {
                eval = new_eval;
                best_move = mov;
            }
            alpha = max(alpha, new_eval);
            if alpha >= beta {
                search_data.ab_cutoffs += 1;
                break;
            }
        }
        Self::store_tt(tt, key, best_move, 0, eval, ply, old_alpha, old_beta);
        eval
    }

    #[inline]
    fn probe_tt(tt: &TranspositionTable, key: u64, moves: &[u32], ply: usize) -> Option<TTEntry> {
        tt.probe(key)
            .filter(|entry| {
                (entry.best_move != NULL_MOVE || entry.depth() == 0)
                    && moves.contains(&entry.best_move)
            })
            .map(|entry| TTEntry {
                score: TranspositionTable::score_from_tt(entry.score, ply as i16),
                ..entry
            })
    }

    #[inline]
    fn store_tt(
        tt: &mut TranspositionTable,
        key: u64,
        best_move: u32,
        depth: u8,
        eval: i16,
        ply: usize,
        old_alpha: i16,
        old_beta: i16,
    ) {
        let bound = if eval <= old_alpha {
            TTEntryType::UpperBound
        } else if eval >= old_beta {
            TTEntryType::LowerBound
        } else {
            TTEntryType::Exact
        };
        tt.store(TTEntry::new_packed(
            key,
            best_move,
            depth,
            bound,
            TranspositionTable::score_to_tt(eval, ply as i16),
            tt.generation,
        ));
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

    ///k == 1, so "selection pick", in place <br>
    ///primary selection and secondary selection are for possible prev pv move and tt move, order depending on tt move depth
    fn partial_selection_sort(
        move_arr_s: &mut [u32],
        primary_selection: u32,
        secondary_selection: u32,
        only_bad_captures_left: &mut Option<bool>,
        move_gen: &MoveGen,
        search_data: &mut SearchData,
        board: &Board
    ) -> u32 {
        let mut best_i: usize = usize::MAX;
        if primary_selection != NULL_MOVE {
            best_i = move_arr_s.iter().position(|m| *m == primary_selection).expect("primary selection was some but not generated");
        } else if secondary_selection != NULL_MOVE {
            best_i = move_arr_s.iter().position(|m| *m == secondary_selection).expect("secondary selection was some but not generated");
        } else { //no priority move available
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
                                cur_v = EATING_MULTIPLIER * PIECE_MATERIAL_VALUE[_move::eaten_piece(mov).unwrap() as usize] as i32;
                            } //else quiets left so ignore bad capture
                        } else { //quiets may remain but we don't know yet
                            cur_v = EATING_MULTIPLIER * PIECE_MATERIAL_VALUE[_move::eaten_piece(mov).unwrap() as usize] as i32;
                        }
                    } else if _move::is_positive_see(mov) {
                        cur_v = GOOD_CAPTURE_BONUS + EATING_MULTIPLIER * PIECE_MATERIAL_VALUE[_move::eaten_piece(mov).unwrap() as usize] as i32;
                        found_dominating = true;
                    } else { //unevaluated capture
                        let is_good_capture: bool =
                            search_data.see_helper.see_positive(mov, board, move_gen);

                        if is_good_capture {
                            move_arr_s[i] = _move::with_positive_see(mov);
                            cur_v = GOOD_CAPTURE_BONUS + EATING_MULTIPLIER * PIECE_MATERIAL_VALUE[_move::eaten_piece(mov).unwrap() as usize] as i32;
                            found_dominating = true;
                        } else {
                            move_arr_s[i] = _move::with_negative_see(mov);
                            if let Some(only_bad_left) = only_bad_captures_left {
                                if *only_bad_left {
                                    cur_v = EATING_MULTIPLIER * PIECE_MATERIAL_VALUE[_move::eaten_piece(mov).unwrap() as usize] as i32;
                                } //else quiets left so ignore bad capture
                            } else { //quiets may remain but we don't know yet
                                cur_v = EATING_MULTIPLIER * PIECE_MATERIAL_VALUE[_move::eaten_piece(mov).unwrap() as usize] as i32;
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

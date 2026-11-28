use std::{
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc, OnceLock,
    },
    time::{Duration, Instant},
};

use crate::{
    repr::{
        _move::{self, *},
        move_gen::MoveGen,
        position::Position,
        types::{WHITE, W_PAWN},
    },
    search::{
        eval::{Evaluator, MATE_BOUND, MATE_EVAL, PIECE_MATERIAL_VALUE},
        move_picker::MovePicker,
        search_config::*,
        search_data::{get_triang_pv_ply_idx_table, SearchData},
        tt::{TTEntry, TTEntryType, TranspositionTable},
    },
    utils::zobrist::Zobrist,
};

pub const MAX_SEARCH_DEPTH: usize = 50;
pub(super) const MAX_PLY: usize = 100;
const THREAD_COUNT: usize = 4;
const STOP_CHECK_INTERVAL: u64 = 2048;
const TIMED_STOP_CHECK_INTERVAL: u64 = 256;
const INFINITY: i16 = MATE_EVAL + 1;
const EVAL_QUIT: i16 = 31111;
const ASPIRATION_DEPTH: usize = 5;
const ASPIRATION_MARGIN: i16 = 32;

struct SearchControl<'a> {
    started: Instant,
    hard_limit: Option<Duration>,
    kill_switch: Option<&'a AtomicBool>,
    ponder_start: Option<&'a OnceLock<Instant>>,
}

impl SearchControl<'_> {
    fn elapsed(&self) -> Option<Duration> {
        match self.ponder_start {
            Some(ponder) => ponder.get().map(Instant::elapsed),
            None => Some(self.started.elapsed()),
        }
    }

    fn deadline_reached(&self) -> bool {
        self.hard_limit
            .is_some_and(|limit| self.elapsed().is_some_and(|elapsed| elapsed >= limit))
    }

    fn should_stop(&self, nodes: u64) -> bool {
        let interval = if self.hard_limit.is_some() {
            TIMED_STOP_CHECK_INTERVAL
        } else {
            STOP_CHECK_INTERVAL
        };
        nodes != 0
            && nodes & (interval - 1) == 0
            && (self.deadline_reached()
                || self.kill_switch.is_some_and(|switch| switch.load(Relaxed)))
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
            Some(bm) if mov.is_some() => !move_eq(bm, mov.unwrap()),
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
            self.search_data[i].board_hash_history.push(new_pos.zhash);

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

    pub fn start_search(
        &mut self,
        move_gen: &MoveGen,
        zobrist: &Zobrist,
        kill_switch: Option<Arc<AtomicBool>>,
    ) {
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

    fn start_search_node(
        &mut self,
        idx: usize,
        move_gen: &MoveGen,
        zobrist: &Zobrist,
        kill_switch: Option<&AtomicBool>,
    ) {
        let (target_depth, hard_ms) = match self.search_config.search_mode {
            SearchMode::StaticDepth(depth) => {
                assert!(
                    depth <= MAX_SEARCH_DEPTH,
                    "static search depth {depth} exceeds MAX_SEARCH_DEPTH {MAX_SEARCH_DEPTH}"
                );
                (depth, None)
            }
            SearchMode::StaticTime(ms) => (MAX_SEARCH_DEPTH, Some(ms)),
            SearchMode::Clock(budget) => (MAX_SEARCH_DEPTH, Some(budget.hard_ms())),
            SearchMode::Infinite => (MAX_SEARCH_DEPTH, None),
        };
        let control = SearchControl {
            started: Instant::now(),
            hard_limit: hard_ms.map(Duration::from_millis),
            kill_switch,
            ponder_start: self.search_config.ponder_start.as_deref(),
        };
        let mut pv_len = self.count_pv_moves(idx);
        let data = &mut self.search_data[idx];
        data.age_history();
        let previous_search_nodes = data.cumul_positions_searched;
        let root_history_index = data.board_hash_history.len().saturating_sub(1);
        let mut search = Search {
            pos: &mut self.positions[idx],
            evaluator: &self.evaluator,
            data,
            move_gen,
            zobrist,
            tt: &mut self.tt,
            control: &control,
            use_quiescence: self.search_config.quiescence,
            previous_pv: [NULL_MOVE; MAX_SEARCH_DEPTH],
            static_evals: [None; MAX_PLY],
            root_history_index,
            repetition_epoch: 0,
            reductions: reductions(),
        };
        let mut last_score: Option<i16> = None;

        for depth in search.data.completed_depth + 1..=target_depth {
            if control.deadline_reached() {
                break;
            }
            search.previous_pv.fill(NULL_MOVE);
            let ordering_len = pv_len.min(depth);
            search.previous_pv[..ordering_len].copy_from_slice(&search.data.pv[..ordering_len]);
            search.data.pv_ply_indices = get_triang_pv_ply_idx_table(depth);

            let mut alpha = -INFINITY;
            let mut beta = INFINITY;
            let mut margin = ASPIRATION_MARGIN;
            if depth >= ASPIRATION_DEPTH && search.use_quiescence {
                if let Some(score) = last_score.filter(|score| score.abs() < MATE_BOUND) {
                    alpha = score - margin;
                    beta = score + margin;
                }
            }

            let eval = loop {
                let eval = search.negamax(depth as i16, 0, alpha, beta, true, true, NULL_MOVE);
                if eval == EVAL_QUIT || (eval > alpha && eval < beta) {
                    break eval;
                }
                if control.deadline_reached() {
                    break EVAL_QUIT;
                }

                margin = margin.saturating_mul(2).min(INFINITY);
                if eval <= alpha {
                    alpha = eval.saturating_sub(margin).max(-INFINITY);
                } else {
                    beta = eval.saturating_add(margin).min(INFINITY);
                }
            };

            search.data.cumul_positions_searched += search.data.positions_searched;
            if eval == EVAL_QUIT {
                if ordering_len != 0 {
                    search.data.pv[..depth].copy_from_slice(&search.previous_pv[..depth]);
                }
                search.data.reset_temp_performance_data();
                break;
            }

            last_score = Some(eval);
            search.data.completed_depth = depth;
            pv_len = search.data.pv[..depth]
                .iter()
                .position(|&mov| mov == NULL_MOVE)
                .unwrap_or(depth);

            if self.search_config.log_uci_diagnostics {
                let (score_kind, score) = if eval.abs() >= MATE_BOUND {
                    let mate_plies = MATE_EVAL - eval.abs();
                    ("mate", ((mate_plies + 1) / 2) * eval.signum())
                } else {
                    ("cp", eval)
                };
                let pv = search.data.pv[..pv_len]
                    .iter()
                    .map(|&mov| _move::to_string(mov, true))
                    .collect::<Vec<_>>()
                    .join(" ");
                let elapsed_ms = control.started.elapsed().as_millis();
                let nodes = search.data.cumul_positions_searched - previous_search_nodes;
                let nps = u128::from(nodes) * 1000 / elapsed_ms.max(1);
                println!(
                    "info depth {depth} seldepth {} score {score_kind} {score} nodes {nodes} nps {nps} time {elapsed_ms} pv {pv}",
                    search.data.sel_depth,
                );
            }

            search.data.reset_temp_performance_data();

            if hard_ms.is_some()
                && control.elapsed().is_some()
                && (pv_len == 0
                    || (eval.abs() >= MATE_BOUND && (MATE_EVAL - eval.abs()) as usize <= depth))
            {
                break;
            }
        }
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
}

struct Search<'a> {
    pos: &'a mut Position,
    evaluator: &'a Evaluator,
    data: &'a mut SearchData,
    move_gen: &'a MoveGen,
    zobrist: &'a Zobrist,
    tt: &'a mut TranspositionTable,
    control: &'a SearchControl<'a>,
    use_quiescence: bool,
    previous_pv: [u32; MAX_SEARCH_DEPTH],
    static_evals: [Option<i16>; MAX_PLY],
    root_history_index: usize,
    repetition_epoch: u64,
    reductions: &'static [[i16; 64]; MAX_SEARCH_DEPTH + 1],
}

impl Search<'_> {
    fn negamax(
        &mut self,
        mut depth: i16,
        ply: usize,
        mut alpha: i16,
        mut beta: i16,
        pv: bool,
        follows_previous_pv: bool,
        skip: u32,
    ) -> i16 {
        self.clear_pv(ply);
        if depth <= 0 && self.use_quiescence {
            return self.quiescence(ply, alpha, beta, pv, None);
        }
        if !self.visit_node(ply) {
            return EVAL_QUIT;
        }

        let (start, end) = self.pos.search_move_bounds();
        let root = ply == 0;
        let in_check = self.pos.board.nof_checkers != 0;
        if start == end {
            return if in_check { -MATE_EVAL + ply as i16 } else { 0 };
        }
        if self.rule_draw(ply) || (!root && self.pos.board.has_insufficient_material()) {
            return 0;
        }
        if depth <= 0 || ply == MAX_PLY {
            return self.raw_eval();
        }

        if !root {
            alpha = alpha.max(-MATE_EVAL + ply as i16);
            beta = beta.min(MATE_EVAL - ply as i16 - 1);
            if alpha >= beta {
                return alpha;
            }
        }

        let key = self.tt_key();
        let excluded = skip != NULL_MOVE;
        let entry = if excluded { None } else { self.probe(key, ply) };
        let safe_clock = self.pos.board.half_move_clock < 90;
        if !pv && safe_clock {
            if let Some(score) = cutoff(entry, depth as u8, alpha, beta) {
                return score;
            }
        }

        self.static_evals[ply] = None;
        let mut raw_eval = 0;
        let mut static_eval = 0;
        let mut improving = false;
        let selective = self.use_quiescence && !root && !in_check && !excluded && safe_clock;
        let non_pawn_material = self.pos.board.has_non_pawn_material();
        let stable_window = alpha.abs() < MATE_BOUND && beta.abs() < MATE_BOUND;

        if !in_check && self.use_quiescence {
            raw_eval = self.raw_eval();
            static_eval = self.correct_eval(raw_eval);
            self.static_evals[ply] = Some(static_eval);
            improving = ply >= 2 && self.static_evals[ply - 2].is_some_and(|old| static_eval > old);
        }

        let epoch = self.repetition_epoch;
        if selective && !pv && stable_window {
            let margin = (if improving { 75 } else { 95 }) * depth;
            if depth <= 6 && non_pawn_material && static_eval >= beta + margin {
                return beta + (static_eval - beta) / 2;
            }

            if depth <= 3 && static_eval < alpha - (245 + 140 * depth * depth) {
                let score = self.quiescence(ply, alpha - 1, alpha, false, Some(static_eval));
                if score == EVAL_QUIT || (score < alpha && score.abs() < MATE_BOUND) {
                    return score;
                }
            }

            if depth >= 3
                && non_pawn_material
                && self.pos.last_target != NULL_MOVE
                && static_eval >= beta
                && !entry.is_some_and(|entry| {
                    entry.bound_type() == TTEntryType::UpperBound && entry.score < beta
                })
            {
                let reduction = 3 + depth / 4 + ((static_eval - beta) / 180).min(2);
                self.pos.make_null_move(self.move_gen, self.zobrist);
                let repetition_start = self.data.repetition_start;
                self.data.repetition_start = self.data.board_hash_history.len();
                let score = -self.negamax(
                    (depth - reduction).max(1),
                    ply + 1,
                    -beta,
                    1 - beta,
                    false,
                    false,
                    NULL_MOVE,
                );
                self.data.repetition_start = repetition_start;
                self.pos.unmake_null_move();
                if score.abs() == EVAL_QUIT {
                    return EVAL_QUIT;
                }
                if score >= beta && score < MATE_BOUND {
                    return score;
                }
            }
        }

        let tt_move = entry.map_or(NULL_MOVE, |hit| hit.best_move);
        if selective && depth >= 6 && tt_move == NULL_MOVE && !follows_previous_pv {
            depth -= 1;
        }
        let can_extend = self.use_quiescence
            && !root
            && !excluded
            && ply + (depth as usize) < self.data.pv_ply_indices.len() + 7
            && depth < MAX_SEARCH_DEPTH as i16;
        if can_extend && in_check && (pv || end - start == 1) {
            depth += 1;
        }
        let mut singular_move = NULL_MOVE;
        if can_extend && selective && depth >= 8 && end - start > 1 {
            if let Some(hit) = entry.filter(|hit| {
                hit.depth() + 3 >= depth as u8
                    && hit.bound_type() != TTEntryType::UpperBound
                    && hit.score.abs() < MATE_BOUND
                    && tt_move != NULL_MOVE
                    && self.pos.move_arr[start..end]
                        .iter()
                        .any(|&mov| move_eq(mov, tt_move))
            }) {
                // Give the TT move extra depth only if every alternative fails low.
                let singular_beta = hit.score - 3 * depth;
                let alternatives = self.negamax(
                    (depth - 1) / 2,
                    ply,
                    singular_beta - 1,
                    singular_beta,
                    false,
                    false,
                    tt_move,
                );
                if alternatives == EVAL_QUIT {
                    return EVAL_QUIT;
                }
                if alternatives < singular_beta {
                    singular_move = tt_move;
                }
            }
        }

        let previous_move = if follows_previous_pv {
            self.previous_pv.get(ply).copied().unwrap_or(NULL_MOVE)
        } else {
            NULL_MOVE
        };
        let (primary, secondary) = if entry.is_some_and(|hit| hit.depth() >= depth as u8) {
            (tt_move, previous_move)
        } else {
            (previous_move, tt_move)
        };
        let mut picker = MovePicker::new(ply, primary, secondary, &self.pos.played_moves_stack);
        let old_alpha = alpha;
        let mut best_score = -INFINITY;
        let mut best_move = NULL_MOVE;
        let mut check_info = None;
        let mut i = start;
        let mut limit = end;
        let side = self.pos.board.turn;
        let futility_margin = 75 + 55 * depth;

        while i < limit {
            let mov = picker.next(
                &mut self.pos.move_arr[start..limit],
                i - start,
                self.move_gen,
                self.data,
                &self.pos.board,
            );
            if move_eq(mov, skip) {
                limit -= 1;
                self.pos.move_arr.swap(i, limit);
                picker.swap_scores(i - start, limit - start);
                continue;
            }
            let quiet = !is_eating(mov) && !is_promotion(mov);
            let priority = move_eq(mov, previous_move) || move_eq(mov, tt_move);
            let advanced_pawn = get_moved_piece(mov) % 6 == W_PAWN
                && if side == WHITE {
                    get_target(mov) >= 40
                } else {
                    get_target(mov) < 24
                };
            let history = if quiet {
                picker.quiet_history(self.data, side, mov)
            } else {
                0
            };
            let played = i - start;
            if selective
                && stable_window
                && !pv
                && played > 0
                && best_score > -MATE_BOUND
                && quiet
                && !priority
                && !advanced_pawn
                && !is_castle(mov)
            {
                let late_limit = (3 + usize::from(improving))
                    + (depth * depth) as usize * if improving { 2 } else { 1 };
                let late = depth <= 5 && played >= late_limit && history <= 0;
                let futile =
                    depth <= 4 && static_eval + futility_margin <= alpha && history < 2_500;
                if late || futile {
                    let info =
                        check_info.get_or_insert_with(|| self.move_gen.check_info(&self.pos.board));
                    if !self.move_gen.gives_check(&self.pos.board, info, mov) {
                        limit -= 1;
                        self.pos.move_arr.swap(i, limit);
                        picker.swap_scores(played, limit - start);
                        continue;
                    }
                }
            }

            let child_depth = depth - 1 + i16::from(move_eq(mov, singular_move));
            let follows = follows_previous_pv && move_eq(mov, previous_move);
            let epoch_before_move = self.repetition_epoch;
            self.make_move(mov, self.use_quiescence && child_depth == 0);
            let reduction = if depth >= 3
                && quiet
                && played >= if pv || root { 3 } else { 1 }
                && !in_check
                && self.pos.board.nof_checkers == 0
                && !priority
                && !advanced_pawn
            {
                (self.reductions[depth as usize][played.min(63)] + i16::from(!pv && !improving)
                    - i16::from(pv)
                    - (history / 6_000).clamp(-1, 1) as i16)
                    .clamp(0, child_depth - 1)
            } else {
                0
            };

            let score = if pv && played == 0 {
                -self.negamax(
                    child_depth,
                    ply + 1,
                    -beta,
                    -alpha,
                    true,
                    follows,
                    NULL_MOVE,
                )
            } else {
                let mut score = -self.negamax(
                    child_depth - reduction,
                    ply + 1,
                    -alpha - 1,
                    -alpha,
                    false,
                    follows,
                    NULL_MOVE,
                );
                if reduction > 0 && score > alpha && score.abs() != EVAL_QUIT {
                    score = -self.negamax(
                        child_depth,
                        ply + 1,
                        -alpha - 1,
                        -alpha,
                        false,
                        follows,
                        NULL_MOVE,
                    );
                }
                if pv && score > alpha && score < beta && score.abs() != EVAL_QUIT {
                    score = -self.negamax(
                        child_depth,
                        ply + 1,
                        -beta,
                        -alpha,
                        true,
                        follows,
                        NULL_MOVE,
                    );
                }
                score
            };
            self.unmake_move(mov);
            if score.abs() == EVAL_QUIT {
                return EVAL_QUIT;
            }

            if score > best_score {
                best_score = score;
                best_move = mov;
                if pv {
                    self.update_pv(ply, mov);
                }
            }
            alpha = alpha.max(score);
            if alpha >= beta {
                self.data.ab_cutoffs += 1;
                if excluded {
                    return score;
                }
                self.data.update_cutoff_history(
                    ply,
                    side,
                    &self.pos.played_moves_stack,
                    mov,
                    &self.pos.move_arr[start..i],
                    depth as usize,
                );
                if self.repetition_epoch == epoch_before_move {
                    self.store(key, mov, depth as u8, score, ply, TTEntryType::LowerBound);
                    if selective && quiet && depth >= 2 && score > static_eval {
                        self.learn_correction(raw_eval, score, depth);
                    }
                }
                return score;
            }
            i += 1;
        }

        if limit != end {
            best_score = best_score.max(old_alpha);
        }
        if !excluded && self.repetition_epoch == epoch {
            let bound = if best_score > old_alpha {
                TTEntryType::Exact
            } else {
                TTEntryType::UpperBound
            };
            self.store(key, best_move, depth as u8, best_score, ply, bound);
            if selective
                && depth >= 2
                && !is_eating(best_move)
                && !is_promotion(best_move)
                && (bound == TTEntryType::Exact || best_score < static_eval)
            {
                self.learn_correction(raw_eval, best_score, depth);
            }
        }
        best_score
    }

    fn quiescence(
        &mut self,
        ply: usize,
        mut alpha: i16,
        beta: i16,
        pv: bool,
        razor_eval: Option<i16>,
    ) -> i16 {
        if !self.visit_node(ply) {
            return EVAL_QUIT;
        }

        let (start, mut end) = self.pos.search_move_bounds();
        let in_check = self.pos.board.nof_checkers != 0;
        if start == end && in_check {
            return -MATE_EVAL + ply as i16;
        }
        if self.rule_draw(ply) || self.pos.board.has_insufficient_material() {
            return 0;
        }

        let key = self.tt_key();
        let entry = self.probe(key, ply);
        if !pv && self.pos.board.half_move_clock < 90 {
            if let Some(score) = cutoff(entry, 0, alpha, beta) {
                return score;
            }
        }

        let old_alpha = alpha;
        let mut best_score = -INFINITY;
        if !in_check {
            if start == end
                && !self
                    .move_gen
                    .has_legal_quiet(&self.pos.board, &mut self.pos.move_generation_temp_arr)
            {
                return 0;
            }
            best_score = razor_eval.unwrap_or_else(|| {
                let raw = self.raw_eval();
                self.correct_eval(raw)
            });
            if best_score >= beta {
                self.data.stand_pat_cutoffs += 1;
                self.store(key, NULL_MOVE, 0, best_score, ply, TTEntryType::LowerBound);
                return best_score;
            }
            alpha = alpha.max(best_score);
        }
        if ply == MAX_PLY {
            return self.raw_eval();
        }

        if razor_eval.is_some() {
            // Preserve full bounds for the main search after a failed razor attempt.
            end = start + partition_noisy(&mut self.pos.move_arr[start..end]);
        }
        if start == end {
            return best_score;
        }

        let tt_move = entry.map_or(NULL_MOVE, |hit| hit.best_move);
        let mut picker = MovePicker::new(ply, tt_move, NULL_MOVE, &self.pos.played_moves_stack);
        let mut best_move = NULL_MOVE;
        let mut check_info = None;
        let epoch = self.repetition_epoch;
        let stand_pat = best_score;
        for i in start..end {
            let mov = picker.next(
                &mut self.pos.move_arr[start..end],
                i - start,
                self.move_gen,
                self.data,
                &self.pos.board,
            );
            if !in_check && !is_promotion(mov) && !is_en_passant(mov) && alpha.abs() < MATE_BOUND {
                let victim = PIECE_MATERIAL_VALUE[eaten_piece(mov).unwrap() as usize];
                let futile =
                    !pv && self.pos.board.major_minor_count > 4 && stand_pat + victim + 220 < alpha;
                if is_negative_see(mov) || futile {
                    let info =
                        check_info.get_or_insert_with(|| self.move_gen.check_info(&self.pos.board));
                    if !self.move_gen.gives_check(&self.pos.board, info, mov) {
                        if futile {
                            best_score = best_score.max(stand_pat + victim + 220);
                        }
                        continue;
                    }
                }
            }
            let epoch_before_move = self.repetition_epoch;
            self.make_move(mov, true);
            let score = -self.quiescence(ply + 1, -beta, -alpha, pv, None);
            self.unmake_move(mov);
            if score.abs() == EVAL_QUIT {
                return EVAL_QUIT;
            }
            if score > best_score {
                best_score = score;
                best_move = mov;
            }
            alpha = alpha.max(score);
            if alpha >= beta {
                self.data.ab_cutoffs += 1;
                if self.repetition_epoch == epoch_before_move {
                    self.store(key, mov, 0, score, ply, TTEntryType::LowerBound);
                }
                return score;
            }
        }

        if self.repetition_epoch == epoch {
            let bound = if best_score > old_alpha {
                TTEntryType::Exact
            } else {
                TTEntryType::UpperBound
            };
            self.store(key, best_move, 0, best_score, ply, bound);
        }
        best_score
    }

    fn visit_node(&mut self, ply: usize) -> bool {
        if self.control.should_stop(self.data.positions_searched) {
            return false;
        }
        self.data.positions_searched += 1;
        self.data.sel_depth = self.data.sel_depth.max(ply);
        true
    }

    fn rule_draw(&mut self, ply: usize) -> bool {
        if self
            .data
            .is_search_repetition(self.pos, self.root_history_index, ply != 0)
        {
            self.repetition_epoch += 1;
            return true;
        }
        self.pos.board.is_fifty_move_draw()
    }

    #[inline]
    fn make_move(&mut self, mov: u32, quiescence: bool) {
        self.pos
            .make_move(mov, true, false, quiescence, self.move_gen, self.zobrist);
        self.data.board_hash_history.push(self.pos.zhash);
    }

    #[inline]
    fn unmake_move(&mut self, mov: u32) {
        self.data.board_hash_history.pop();
        self.pos.unmake_move(mov, self.zobrist);
    }

    #[inline]
    fn raw_eval(&self) -> i16 {
        self.evaluator
            .evaluate_position(&self.pos.board, self.move_gen)
    }

    #[inline]
    fn correct_eval(&self, raw: i16) -> i16 {
        if self.pos.board.major_minor_count == 0 {
            raw
        } else {
            self.data.correct_static_eval(&self.pos.board, raw)
        }
    }

    fn learn_correction(&mut self, raw: i16, score: i16, depth: i16) {
        if self.pos.board.major_minor_count > 0 && score.abs() < MATE_BOUND {
            self.data
                .update_eval_correction(&self.pos.board, raw, score, depth as usize);
        }
    }

    fn clear_pv(&mut self, ply: usize) {
        if ply + 1 < self.data.pv_ply_indices.len() {
            self.data.pv[self.data.pv_ply_indices[ply]] = NULL_MOVE;
        }
    }

    fn update_pv(&mut self, ply: usize, mov: u32) {
        let indices = &self.data.pv_ply_indices;
        if ply + 1 < indices.len() {
            let row_start = indices[ply];
            self.data.pv[row_start] = mov;
            if ply + 2 < indices.len() {
                self.data
                    .pv
                    .copy_within(indices[ply + 1]..indices[ply + 2], row_start + 1);
            }
        }
    }

    #[inline]
    fn tt_key(&self) -> u64 {
        Zobrist::zkey50_adjusted(self.pos.zhash, self.pos.board.half_move_clock)
    }

    #[inline]
    fn probe(&mut self, key: u64, ply: usize) -> Option<TTEntry> {
        self.tt.probe_refresh(key).map(|entry| TTEntry {
            score: TranspositionTable::score_from_tt(entry.score, ply as i16),
            ..entry
        })
    }

    #[inline]
    fn store(&mut self, key: u64, mov: u32, depth: u8, score: i16, ply: usize, bound: TTEntryType) {
        self.tt.store(TTEntry::new_packed(
            key,
            mov,
            depth,
            bound,
            TranspositionTable::score_to_tt(score, ply as i16),
            self.tt.generation,
        ));
    }
}

fn cutoff(entry: Option<TTEntry>, depth: u8, alpha: i16, beta: i16) -> Option<i16> {
    let entry = entry.filter(|entry| entry.depth() >= depth)?;
    let score = entry.score;
    match entry.bound_type() {
        TTEntryType::Exact => Some(score),
        TTEntryType::LowerBound if score >= beta => Some(score),
        TTEntryType::UpperBound if score <= alpha => Some(score),
        _ => None,
    }
}

fn partition_noisy(moves: &mut [u32]) -> usize {
    let mut count = 0;
    for i in 0..moves.len() {
        if is_eating(moves[i]) || is_promotion(moves[i]) {
            moves.swap(i, count);
            count += 1;
        }
    }
    count
}

fn reductions() -> &'static [[i16; 64]; MAX_SEARCH_DEPTH + 1] {
    static TABLE: OnceLock<[[i16; 64]; MAX_SEARCH_DEPTH + 1]> = OnceLock::new();
    TABLE.get_or_init(|| {
        std::array::from_fn(|depth| {
            std::array::from_fn(|played| {
                (0.55 + (depth.max(1) as f32).ln() * ((played + 1) as f32).ln() / 2.4) as i16
            })
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiesce(fen: &str, razor_eval: Option<i16>) -> (i16, u64) {
        let move_gen = MoveGen::init();
        let zobrist = Zobrist::default();
        let mut pos = Position::from(fen, &move_gen, &zobrist).unwrap();
        let mut data = SearchData::new(&pos);
        let mut search = Search {
            pos: &mut pos,
            evaluator: &Evaluator::default(),
            data: &mut data,
            move_gen: &move_gen,
            zobrist: &zobrist,
            tt: &mut TranspositionTable::default(),
            control: &SearchControl {
                started: Instant::now(),
                hard_limit: None,
                kill_switch: None,
                ponder_start: None,
            },
            use_quiescence: true,
            previous_pv: [NULL_MOVE; MAX_SEARCH_DEPTH],
            static_evals: [None; MAX_PLY],
            root_history_index: 0,
            repetition_epoch: 0,
            reductions: reductions(),
        };
        let bounds = search.pos.search_move_bounds();
        let score = search.quiescence(0, -1, 0, false, razor_eval);
        assert_eq!(search.pos.search_move_bounds(), bounds);
        (score, data.positions_searched)
    }

    #[test]
    fn razor_probe_searches_captures_and_keeps_full_move_bounds() {
        assert_eq!(
            quiesce("4k3/8/8/8/8/8/1q6/R3K3 w - - 0 1", Some(-500)),
            (-500, 1)
        );
        let (score, nodes) = quiesce("4k3/q7/8/8/8/8/8/R3K3 w - - 0 1", Some(-500));
        assert!(score > 0 && nodes > 1);
    }

    #[test]
    fn quiescence_recognizes_stalemate_with_no_noisy_moves() {
        assert_eq!(quiesce("7k/5K2/6Q1/8/8/8/8/8 b - - 0 1", None), (0, 1));
    }
}

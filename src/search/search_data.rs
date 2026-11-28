use crate::{
    repr::{
        _move::{self, NULL_MOVE},
        board::Board,
        position::Position,
        types::{B_PAWN_U, W_PAWN_U},
    },
    search::{
        eval::MATE_BOUND, legacy_see::LegacySeeWorker, searcher::MAX_SEARCH_DEPTH, see::SeeWorker,
    },
};

pub const TRIANG_PV_TABLE_SIZE: usize = (MAX_SEARCH_DEPTH * (MAX_SEARCH_DEPTH + 1)) / 2;
const MAX_HISTORY_VAL: i32 = 7183; //from stockfish
const BONUS_MULTIPLIER: i32 = 7;
const HISTORY_AGING_NUMERATOR: i32 = 3;
const HISTORY_AGING_DENOMINATOR: i32 = 4;
const CORRECTION_BITS: u32 = 13;
const CORRECTION_ENTRIES: usize = 1 << CORRECTION_BITS;
const CORRECTION_SCALE: i32 = 64;
const MAX_CORRECTION: i32 = 128 * CORRECTION_SCALE;

pub struct SearchData {
    // Triangular scratch/result table. The root PV starts at index 0 and never
    // contains quiescence moves. It can be partial after interruption; its length
    // does not establish completed search depth.
    pub pv: [u32; TRIANG_PV_TABLE_SIZE],
    /// Nominal depth of the last completed iteration, independent of PV length.
    pub completed_depth: usize,
    // Start of every ply row followed by the final one-past-end table boundary.
    pub pv_ply_indices: Vec<usize>,
    pub mate_in: Option<u32>,
    pub board_hash_history: Vec<u64>, //only relevant, i.e. since last non-reversible move
    pub(super) repetition_start: usize,
    pub history_table: [i16; 2 * 64 * 64], //history_table[side * 4096 + from_sq * 64 + to_sq]
    correction_history: Box<[i16]>,
    pub(super) legacy_see: LegacySeeWorker,
    pub see_helper: SeeWorker,
    //per search data
    pub positions_searched: u64,
    pub stand_pat_cutoffs: u64,
    pub ab_cutoffs: u64,
    pub sel_depth: usize,
    //cumulative data
    pub cumul_positions_searched: u64,
}

impl SearchData {
    /// Between games reset
    pub fn reset(&mut self) {
        self.pv.fill(NULL_MOVE);
        self.completed_depth = 0;
        self.pv_ply_indices = get_triang_pv_ply_idx_table(1);
        self.history_table.fill(0);
        self.correction_history.fill(0);
        self.mate_in = None;
        self.repetition_start = 0;
        self.reset_temp_performance_data();
        self.reset_cumul_performance_data();
    }

    pub fn new(pos: &Position) -> Self {
        let mut board_hash_history: Vec<u64> = Vec::with_capacity(32);
        board_hash_history.push(pos.zhash);
        return Self {
            pv: [NULL_MOVE; TRIANG_PV_TABLE_SIZE],
            completed_depth: 0,
            pv_ply_indices: get_triang_pv_ply_idx_table(1),
            mate_in: None,
            board_hash_history: board_hash_history,
            repetition_start: 0,
            history_table: [0; 2 * 64 * 64],
            correction_history: vec![0; 2 * CORRECTION_ENTRIES].into_boxed_slice(),
            legacy_see: LegacySeeWorker::default(),
            see_helper: SeeWorker,
            positions_searched: 0,
            stand_pat_cutoffs: 0,
            ab_cutoffs: 0,
            sel_depth: 0,
            cumul_positions_searched: 0,
        };
    }

    pub fn with_board_hash_history(pos: &Position, board_hash_history: Vec<u64>) -> Self {
        Self {
            board_hash_history,
            ..Self::new(pos)
        }
    }

    pub fn in_three_fold(&self, pos: &Position) -> bool {
        let mut count: u32 = 1;
        let e = self.board_hash_history.len().saturating_sub(1);
        let mut i = self.repetition_start + ((e ^ self.repetition_start) & 1);
        while i < e {
            if pos.zhash == self.board_hash_history[i] {
                count += 1;
            }
            i += 2;
        }
        return count >= 3;
    }

    pub(super) fn is_search_repetition(
        &self,
        pos: &Position,
        root: usize,
        below_root: bool,
    ) -> bool {
        let Some(current) = self.board_hash_history.len().checked_sub(1) else {
            return false;
        };
        let start = self
            .repetition_start
            .max(current.saturating_sub(pos.board.half_move_clock as usize));
        let mut found = false;
        // The shortest legal return to a position takes four plies.
        let mut previous = current.checked_sub(4);
        while let Some(i) = previous.filter(|&i| i >= start) {
            if self.board_hash_history[i] == pos.zhash {
                if (below_root && i >= root) || found {
                    return true;
                }
                found = true;
            }
            previous = i.checked_sub(2);
        }
        false
    }

    pub fn reset_temp_performance_data(&mut self) {
        self.positions_searched = 0;
        self.ab_cutoffs = 0;
        self.sel_depth = 0;
        self.stand_pat_cutoffs = 0;
    }

    pub fn reset_cumul_performance_data(&mut self) {
        self.cumul_positions_searched = 0;
    }

    pub(crate) fn age_history(&mut self) {
        for entry in &mut self.history_table {
            *entry = (*entry as i32 * HISTORY_AGING_NUMERATOR / HISTORY_AGING_DENOMINATOR) as i16;
        }
    }

    #[inline]
    pub fn update_history_entry(&mut self, side: u32, from: u32, to: u32, bonus: i32) {
        let idx: usize = (side * 4096 + from * 64 + to) as usize;
        Self::update_history_value(&mut self.history_table[idx], bonus);
    }

    #[inline]
    pub fn get_history_entry(&self, side: u32, mov: u32) -> i32 {
        let idx: usize =
            (side * 4096 + _move::get_init(mov) * 64 + _move::get_target(mov)) as usize;
        return self.history_table[idx] as i32;
    }

    pub(super) fn update_cutoff_history(
        &mut self,
        side: u32,
        cutoff_move: u32,
        tried_moves: &[u32],
        depth: usize,
    ) {
        if _move::is_eating(cutoff_move) || _move::is_promotion(cutoff_move) {
            return;
        }
        let bonus = depth as i32;
        self.update_history_entry(
            side,
            _move::get_init(cutoff_move),
            _move::get_target(cutoff_move),
            bonus,
        );
        for &mov in tried_moves {
            if !_move::is_eating(mov) && !_move::is_promotion(mov) {
                self.update_history_entry(
                    side,
                    _move::get_init(mov),
                    _move::get_target(mov),
                    -bonus,
                );
            }
        }
    }

    #[inline]
    pub(super) fn correct_static_eval(&self, board: &Board, raw_score: i16) -> i16 {
        let correction = i32::from(self.correction_history[Self::correction_index(board)]);
        (i32::from(raw_score) + correction / CORRECTION_SCALE)
            .clamp(i32::from(-MATE_BOUND + 1), i32::from(MATE_BOUND - 1)) as i16
    }

    pub(super) fn update_eval_correction(
        &mut self,
        board: &Board,
        raw_score: i16,
        result: i16,
        depth: usize,
    ) {
        let index = Self::correction_index(board);
        let entry = i32::from(self.correction_history[index]);
        let target = ((i32::from(result) - i32::from(raw_score)) * CORRECTION_SCALE)
            .clamp(-2 * MAX_CORRECTION, 2 * MAX_CORRECTION);
        let weight = (depth + 1).min(16) as i32;
        self.correction_history[index] =
            (entry + (target - entry) * weight / 128).clamp(-MAX_CORRECTION, MAX_CORRECTION) as i16;
    }

    #[inline]
    fn correction_index(board: &Board) -> usize {
        let white = board.pieces[W_PAWN_U].wrapping_mul(0x9e3779b97f4a7c15);
        let black = board.pieces[B_PAWN_U].wrapping_mul(0xbf58476d1ce4e5b9);
        let hash = white ^ black.rotate_left(17);
        let mixed = (hash ^ (hash >> 32)).wrapping_mul(0x94d049bb133111eb);
        (mixed >> (64 - CORRECTION_BITS)) as usize + board.turn as usize * CORRECTION_ENTRIES
    }

    #[inline]
    fn update_history_value(entry: &mut i16, bonus: i32) {
        let bonus = bonus
            .saturating_mul(BONUS_MULTIPLIER)
            .clamp(-MAX_HISTORY_VAL, MAX_HISTORY_VAL);
        let value = i32::from(*entry);
        *entry = (value + bonus - value * bonus.abs() / MAX_HISTORY_VAL) as i16;
    }
}

pub fn get_triang_pv_ply_idx_table(target_d: usize) -> Vec<usize> {
    let mut pv_ply_indices: Vec<usize> = Vec::with_capacity(target_d + 1);
    let mut cumul: usize = 0;
    for i in (1..=target_d).rev() {
        pv_ply_indices.push(cumul);
        cumul += i;
    }
    pv_ply_indices.push(cumul);
    return pv_ply_indices;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{repr::move_gen::MoveGen, utils::zobrist::Zobrist};

    #[test]
    fn search_repetition_distinguishes_played_history_from_search_cycles() {
        let mut pos = Position::default(&MoveGen::init(), &Zobrist::default());
        pos.board.half_move_clock = 8;
        let hash = pos.zhash;
        let other = !hash;
        let mut data =
            SearchData::with_board_hash_history(&pos, vec![hash, other, other, other, hash]);
        assert!(data.is_search_repetition(&pos, 0, true));
        assert!(!data.is_search_repetition(&pos, 4, true));
        assert!(!data.is_search_repetition(&pos, 0, false));
        data.board_hash_history.extend([other, other, other, hash]);
        assert!(data.is_search_repetition(&pos, 8, false));
        data.repetition_start = 1;
        assert!(!data.is_search_repetition(&pos, 8, false));
    }

    #[test]
    fn repetition_scan_stops_at_null_move_boundary() {
        let pos = Position::default(&MoveGen::init(), &Zobrist::default());
        let hash = pos.zhash;
        let other = !hash;
        let mut data =
            SearchData::with_board_hash_history(&pos, vec![hash, other, hash, other, hash]);
        assert!(data.in_three_fold(&pos));
        data.repetition_start = 3;
        assert!(!data.in_three_fold(&pos));
        data.board_hash_history.extend([other, hash, other, hash]);
        assert!(data.in_three_fold(&pos));
    }

    #[test]
    fn evaluation_correction_is_bounded_and_specific_to_pawns_and_mover() {
        let pos = Position::default(&MoveGen::init(), &Zobrist::default());
        let mut data = SearchData::new(&pos);
        let mut other_pawns = pos.board.clone();
        other_pawns.pieces[W_PAWN_U] ^= 1 << 52;
        let mut other_mover = pos.board.clone();
        other_mover.turn ^= 1;

        for _ in 0..32 {
            data.update_eval_correction(&pos.board, 0, 1_000, 10);
        }

        assert_eq!(data.correct_static_eval(&pos.board, 20), 148);
        assert_eq!(data.correct_static_eval(&other_pawns, 20), 20);
        assert_eq!(data.correct_static_eval(&other_mover, 20), 20);
        data.reset();
        assert_eq!(data.correct_static_eval(&pos.board, 20), 20);
    }
}

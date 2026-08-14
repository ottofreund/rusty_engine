use crate::{
    repr::{
        _move::{self, NULL_MOVE},
        position::Position,
        types::NOF_PIECE_TYPES_U,
    },
    search::{searcher::MAX_SEARCH_DEPTH, see::SeeWorker},
};

pub const TRIANG_PV_TABLE_SIZE: usize = (MAX_SEARCH_DEPTH * (MAX_SEARCH_DEPTH + 1)) / 2;
const QUIET_HISTORY_MAX: i32 = 7183;
const QUIET_HISTORY_BONUS_MULTIPLIER: i32 = 7;
const CAPTURE_HISTORY_MAX: i32 = 7183;
const CAPTURE_HISTORY_BONUS_MULTIPLIER: i32 = 7;
const CAPTURE_HISTORY_MALUS_MULTIPLIER: i32 = 7;
const HISTORY_AGING_NUMERATOR: i32 = 3;
const HISTORY_AGING_DENOMINATOR: i32 = 4;
const PIECE_COUNT: usize = 2 * NOF_PIECE_TYPES_U;
pub const CAPTURE_HISTORY_TABLE_SIZE: usize = PIECE_COUNT * 64 * NOF_PIECE_TYPES_U;

pub struct SearchData {
    // Triangular scratch/result table. The completed root PV always starts at index 0
    // and never contains quiescence moves.
    pub pv: [u32; TRIANG_PV_TABLE_SIZE],
    // Start of every ply row followed by the final one-past-end table boundary.
    pub pv_ply_indices: Vec<usize>,
    pub mate_in: Option<u32>,
    pub board_hash_history: Vec<u64>, //only relevant, i.e. since last non-reversible move
    pub history_table: [i16; 2 * 64 * 64], //history_table[side * 4096 + from_sq * 64 + to_sq]
    // capture_history_table[moving_piece * 384 + target_sq * 6 + captured_piece_type]
    pub capture_history_table: [i16; CAPTURE_HISTORY_TABLE_SIZE],
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
    pub fn new(pos: &Position) -> Self {
        let mut board_hash_history: Vec<u64> = Vec::with_capacity(32);
        board_hash_history.push(pos.board.zhash);
        return Self {
            pv: [NULL_MOVE; TRIANG_PV_TABLE_SIZE],
            pv_ply_indices: get_triang_pv_ply_idx_table(1),
            mate_in: None,
            board_hash_history: board_hash_history,
            history_table: [0; 2 * 64 * 64],
            capture_history_table: [0; CAPTURE_HISTORY_TABLE_SIZE],
            see_helper: SeeWorker::default(),
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
        let mut i: usize;
        if self.board_hash_history.len() % 2 == 0 {
            i = 1;
        } else {
            i = 0;
        }
        let e: usize = self.board_hash_history.len() - 1;
        while i < e {
            if pos.board.zhash == self.board_hash_history[i] {
                count += 1;
            }
            i += 2;
        }
        return count >= 3;
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
        for entry in self.history_table.iter_mut().chain(self.capture_history_table.iter_mut()) {
            *entry = (*entry as i32 * HISTORY_AGING_NUMERATOR
                / HISTORY_AGING_DENOMINATOR) as i16;
        }
    }

    #[inline]
    pub fn update_history_entry(&mut self, side: u32, from: u32, to: u32, bonus: i32) {
        let idx: usize = (side * 4096 + from * 64 + to) as usize;
        Self::update_bounded_history(
            &mut self.history_table[idx],
            bonus,
            QUIET_HISTORY_BONUS_MULTIPLIER,
            QUIET_HISTORY_MAX,
        );
    }

    #[inline]
    pub fn get_history_entry(&self, side: u32, mov: u32) -> i32 {
        let idx: usize = (side * 4096 + _move::get_init(mov) * 64 + _move::get_target(mov)) as usize;
        return self.history_table[idx] as i32;
    }

    #[inline]
    pub fn update_capture_history_entry(&mut self, mov: u32, bonus: i32) {
        if !_move::is_eating(mov) || _move::is_promotion(mov) {
            return;
        }

        let idx = Self::capture_history_idx(mov);
        let multiplier = if bonus >= 0 {
            CAPTURE_HISTORY_BONUS_MULTIPLIER
        } else {
            CAPTURE_HISTORY_MALUS_MULTIPLIER
        };
        Self::update_bounded_history(
            &mut self.capture_history_table[idx],
            bonus,
            multiplier,
            CAPTURE_HISTORY_MAX,
        );
    }

    #[inline]
    pub fn get_capture_history_entry(&self, mov: u32) -> i32 {
        if !_move::is_eating(mov) || _move::is_promotion(mov) {
            return 0;
        }

        return self.capture_history_table[Self::capture_history_idx(mov)] as i32;
    }

    #[inline]
    fn capture_history_idx(mov: u32) -> usize {
        let moving_piece = _move::get_moved_piece(mov) as usize;
        let target = _move::get_target(mov) as usize;
        let captured_piece_type = _move::eaten_piece(mov)
            .expect("capture history move had no captured piece") as usize
            % NOF_PIECE_TYPES_U;

        return moving_piece * 64 * NOF_PIECE_TYPES_U
            + target * NOF_PIECE_TYPES_U
            + captured_piece_type;
    }

    #[inline]
    fn update_bounded_history(entry: &mut i16, bonus: i32, multiplier: i32, max: i32) {
        let scaled_bonus = (multiplier * bonus).clamp(-max, max);
        let current = *entry as i32;
        let updated = current + scaled_bonus - current * scaled_bonus.abs() / max;
        debug_assert!(updated.abs() <= max);
        *entry = updated as i16;
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
    return pv_ply_indices
}

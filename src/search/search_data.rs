use crate::{
    repr::{_move::NULL_MOVE, position::Position}, search::{searcher::MAX_SEARCH_DEPTH, see::SeeWorker},
};

pub const TRIANG_PV_TABLE_SIZE: usize = (MAX_SEARCH_DEPTH * (MAX_SEARCH_DEPTH + 1)) / 2;

pub struct SearchData {
    // Triangular scratch/result table. The completed root PV always starts at index 0
    // and never contains quiescence moves.
    pub pv: [u32; TRIANG_PV_TABLE_SIZE],
    // Start of every ply row followed by the final one-past-end table boundary.
    pub pv_ply_indices: Vec<usize>,
    pub mate_in: Option<u32>,
    pub board_hash_history: Vec<u64>, //only relevant, i.e. since last non-reversible move
    pub see_helper: SeeWorker,
    pub positions_searched: u64,      //per search
    pub ab_cutoffs: u64,
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
            see_helper: SeeWorker::default(),
            positions_searched: 0,
            ab_cutoffs: 0,
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

    pub fn log_performance(&self) {
        println!(
            "positions searched: {}, ab cutoffs: {}",
            self.positions_searched, self.ab_cutoffs
        );
    }

    pub fn reset_temp_performance_data(&mut self) {
        self.positions_searched = 0;
        self.ab_cutoffs = 0;
    }

    pub fn reset_cumul_performance_data(&mut self) {
        self.cumul_positions_searched = 0;
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

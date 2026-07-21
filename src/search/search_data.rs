use crate::{
    repr::{_move::NULL_MOVE, position::Position},
    search::searcher::MAX_SEARCH_DEPTH,
};

pub struct SearchData {
    pub pv: [u32; MAX_SEARCH_DEPTH + 1], //next moves on last search's primary variation (i.e. search result), doesn't contain quiescence moves
    pub mate_in: Option<u32>,
    pub board_hash_history: Vec<u64>, //only relevant, i.e. since last non-reversible move
    pub positions_searched: u64,      //per search
    pub ab_cutoffs: u64,
    pub cumul_positions_searched: u64,
}

impl SearchData {
    pub fn new(pos: &Position) -> Self {
        let mut board_hash_history: Vec<u64> = Vec::with_capacity(32);
        board_hash_history.push(pos.board.zhash);
        return Self {
            pv: [NULL_MOVE; MAX_SEARCH_DEPTH + 1],
            mate_in: None,
            board_hash_history: board_hash_history,
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

use std::collections::HashMap;

use crate::{repr::{_move::NULL_MOVE, position::Position}, search::searcher::MAX_SEARCH_DEPTH};




pub struct SearchData {
    pub pv_move: [u32 ; MAX_SEARCH_DEPTH + 1], //next moves on last search's primary variation (i.e. search result)
    pub mate_in: Option<u32>,
    //zobrist hash value to nof times position was reached
    //a collision may cause inadvertent three-fold-repetition, doesn't affect correctness in search though:
    //the unlikely worst case it that the engine seeks improper three-fold in an already losing position
    pub repetition_map: HashMap<u64, u32>, 
    pub positions_searched: u64, //per search
    pub ab_cutoffs: u64,
    pub cumul_positions_searched: u64
}

impl SearchData {

    pub fn new(pos: &Position) -> Self {
        let mut repetition_map: HashMap<u64, u32> = HashMap::new();
        repetition_map.insert(pos.board.zhash, 1);
        return Self { 
            pv_move: [NULL_MOVE ; MAX_SEARCH_DEPTH + 1], 
            mate_in: None,
            repetition_map,
            positions_searched: 0,
            ab_cutoffs: 0,
            cumul_positions_searched: 0
        };
    }

    pub fn in_three_fold(&self, pos: &Position, is_cur_pos: bool) -> bool {
        match self.repetition_map.get(&pos.board.zhash) {
            Some(r) => {
                if *r >= 3 {
                    return true; 
                } else {
                    return false;
                }
            },
            None => {
                if is_cur_pos {
                    panic!("cur pos wasn't in repetition_map");
                } else {
                    return false;
                }
            }
        }
    }

    pub fn log_performance(&self) {
        println!("positions searched: {}, ab cutoffs: {}", self.positions_searched, self.ab_cutoffs);
    }

    pub fn reset_temp_performance_data(&mut self) {
        self.positions_searched = 0;
        self.ab_cutoffs = 0;
    }

    pub fn reset_cumul_performance_data(&mut self) {
        self.cumul_positions_searched = 0;
    }

}

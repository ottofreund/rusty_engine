use std::collections::HashMap;

use crate::{repr::_move::NULL_MOVE, search::{searcher::MAX_SEARCH_DEPTH}};




pub struct SearchData {
    pub pv_move: [u32 ; MAX_SEARCH_DEPTH + 1], //next moves on last search's primary variation (i.e. search result)
    pub mate_in: Option<u32>,
    //zobrist hash value to nof times position was reached
    //known deficiency is that a collision may cause inadvertent three-fold-repetition
    //this however is very unlikely to actually happen on board, (more in search, though could cause engine to seek improper three-fold in a losing position)
    pub repetition_map: HashMap<u64, u32>, 
    pub fifty_move_counter: u32,
    pub positions_searched: u64,
    pub ab_cutoffs: u64
}

impl SearchData {
    pub fn log_performance(&self) {
        println!("positions searched: {}, ab cutoffs: {}", self.positions_searched, self.ab_cutoffs);
    }

    pub fn reset_performance_data(&mut self) {
        self.positions_searched = 0;
        self.ab_cutoffs = 0;
    }

}

impl Default for SearchData {
    fn default() -> Self {
        return Self { 
            pv_move: [NULL_MOVE ; MAX_SEARCH_DEPTH + 1], 
            mate_in: None,
            repetition_map: HashMap::<u64, u32>::new(),
            fifty_move_counter: 0,
            positions_searched: 0,
            ab_cutoffs: 0
        };
    }
}
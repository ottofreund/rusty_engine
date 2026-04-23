use std::collections::HashMap;

use crate::{repr::_move::NULL_MOVE, search::{search::MAX_SEARCH_DEPTH}};




pub struct SearchData {
    pub pv_move: [u32 ; MAX_SEARCH_DEPTH + 1], //next moves on last search's primary variation (i.e. search result)
    pub mate_in: Option<u32>,
    //zobrist hash value to nof times position was reached
    //known deficiency is that a collision may cause inadvertent three-fold-repetition
    //this however is very unlikely to actually happen on board, (more in search, though could cause engine to seek improper three-fold in a losing position)
    pub repetition_map: HashMap<u64, u32>, 
    pub fifty_move_counter: u32
}

impl Default for SearchData {
    fn default() -> Self {
        return Self { 
            pv_move: [NULL_MOVE ; MAX_SEARCH_DEPTH + 1], 
            mate_in: None,
            repetition_map: HashMap::<u64, u32>::new(),
            fifty_move_counter: 0
        };
    }
}
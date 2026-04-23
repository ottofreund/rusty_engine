use crate::{repr::{move_gen::MoveGen, position::Position}, search::{search::Searcher}};



pub struct Game {
    pub position: Position,
    pub searcher: Searcher,
    pub move_gen: MoveGen
}

impl Default for Game {
    fn default() -> Self {
        let move_gen: MoveGen = MoveGen::init();
        let position: Position = Position::default(&move_gen);
        let searcher: Searcher = Searcher::from(&position);
        return Self {
            position,
            searcher,
            move_gen
        }
    }
}
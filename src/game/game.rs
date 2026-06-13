use crate::{repr::{move_gen::MoveGen, position::Position}, search::{searcher::Searcher}};



pub struct Game {
    pub position: Position,
    pub searcher: Searcher,
    pub move_gen: MoveGen
}

impl Game {

    ///also imports to searcher to stay in sync
    pub fn import_position(&mut self, position: Position) {
        self.searcher.import_position(&position);
        self.position = position;
    }
    
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
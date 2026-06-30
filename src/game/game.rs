use crate::{repr::{_move, move_gen::MoveGen, position::Position}, search::searcher::Searcher, utils::zobrist::Zobrist};
use std::fmt::Error;


pub struct Game {
    pub position: Position,
    pub searcher: Searcher,
    pub move_gen: MoveGen,
    pub zobrist: Zobrist
}

impl Game {

    ///also imports to searcher to stay in sync
    pub fn import_position(&mut self, position: Position) {
        self.searcher.import_position(&position);
        self.position = position;
    }

    ///Public api ease of use and safety method, not called in search
    ///Returns Success(made_move) if successful
    pub fn try_make_move(&mut self, init_sqr: u32, target_sqr: u32) -> Result<u32, Error> {
        let mov: Option<u32> = self.position.legal_moves().iter().copied().find(|mov| 
            _move::get_init(*mov) == init_sqr && _move::get_target(*mov) == target_sqr
        );
        match mov {
            Some(m) => {
                //println!("Successfully moved: {}", _move::to_string(m));
                self.position.make_move(m, false, false, &self.move_gen, &self.zobrist);
                self.searcher.sync_new_move(&self.position);
                return Ok(m);
            },
            None => {
                return Err(Error::default())
            }
        }
    }

    /* ///Public api ease of use and safety method DEPRECATED
    pub fn try_unmake_move(&mut self) -> Result<u32, Error> {
        let mov: Option<u32> = self.played_moves_stack.last().copied();
        match mov {
            Some(m) => {
                println!("Successfully unmade: {}", _move::to_string(m));
                self.unmake_move(m);
                return Ok(m);
            },
            None => {
                println!("Tried to unmake move with no moves played");
                return Err(Error::default());
            }
        }
    } */

    
}

impl Default for Game {
    fn default() -> Self {
        let move_gen: MoveGen = MoveGen::init();
        let zobrist: Zobrist = Zobrist::default();
        let position: Position = Position::default(&move_gen, &zobrist);
        let searcher: Searcher = Searcher::from(&position);
        
        return Self {
            position,
            searcher,
            move_gen,
            zobrist
        }
    }
}
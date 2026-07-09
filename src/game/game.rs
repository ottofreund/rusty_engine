use crate::{
    game::game_state::GameState,
    repr::{_move, board::Board, move_gen::MoveGen, position::Position},
    search::searcher::Searcher,
    utils::zobrist::Zobrist,
};
use std::fmt::Error;

///Game object that is also compatible for UI on-board-games
pub struct Game {
    pub position: Position,
    pub searcher: Searcher,
    pub move_gen: MoveGen,
    pub zobrist: Zobrist,
    pub game_state: GameState,
    pub board_history: Vec<Board>,
    repetition_relevant_history_idx: usize, // [this..] are relevant for checking repetition
}

impl Game {
    ///also imports to searcher to stay in sync
    pub fn import_position(&mut self, position: Position) {
        self.searcher.import_position(&position, None);
        self.position = position;
        self.board_history.clear();
        self.board_history.push(self.position.board.clone());
        self.repetition_relevant_history_idx = 0;
        self.game_state = self.current_game_state();
    }

    ///For making moves on the board, not called in search
    ///Syncs the searcher if successful
    ///Returns Success(made_move) if successful
    pub fn try_make_move(&mut self, init_sqr: u32, target_sqr: u32) -> Result<u32, Error> {
        let mov: Option<u32> =
            self.position.legal_moves().iter().copied().find(|mov| {
                _move::get_init(*mov) == init_sqr && _move::get_target(*mov) == target_sqr
            });
        match mov {
            Some(m) => {
                //println!("Successfully moved: {}", _move::to_string(m, true));
                self.position
                    .make_move(m, false, false, &self.move_gen, &self.zobrist);
                self.searcher.sync_new_move(&self.position, Some(m));
                if _move::is_unrepeatable(m) {
                    println!("move was unrepeatable");
                    self.repetition_relevant_history_idx = self.board_history.len();
                }
                self.board_history.push(self.position.board.clone());
                self.game_state = self.current_game_state();
                println!("Game state: {}", self.game_state.to_string());
                return Ok(m);
            }
            None => return Err(Error::default()),
        }
    }

    fn current_game_state(&self) -> GameState {
        if self.position.in_checkmate() {
            return GameState::Checkmate(self.position.board.turn);
        } else if self.position.in_stalemate() {
            return GameState::Stalemate;
        } else if self.cur_pos_is_threefold() {
            return GameState::DrawByRepetition;
        } else if self.position.board.is_fifty_move_draw() {
            return GameState::DrawByFiftyMoveRule;
        } else {
            return GameState::InProgress;
        }
    }

    fn cur_pos_is_threefold(&self) -> bool {
        let mut count: u32 = 1;
        for i in self.repetition_relevant_history_idx..(self.board_history.len() - 1) {
            if self.position.board == self.board_history[i] {
                count += 1;
            }
        }
        return count >= 3;
    }

    /* ///Public api ease of use and safety method DEPRECATED
    pub fn try_unmake_move(&mut self) -> Result<u32, Error> {
        let mov: Option<u32> = self.played_moves_stack.last().copied();
        match mov {
            Some(m) => {
                println!("Successfully unmade: {}", _move::to_string(m, true));
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
        let board_history: Vec<Board> = vec![position.board.clone()];

        return Self {
            position,
            searcher,
            move_gen,
            zobrist,
            game_state: GameState::InProgress,
            board_history,
            repetition_relevant_history_idx: 0,
        };
    }
}

use crate::repr::{board::Board, move_gen::MoveGen, types::Color};


pub struct Game {
    pub board: Board,
    pub move_gen: MoveGen,
    pub legal_moves: Vec<u32>
}

impl Game {
    pub fn init_default() -> Game {
        let move_gen: MoveGen = MoveGen::init();
        let mut board: Board = Board::default_board(&move_gen);
        let turn: Color = board.turn.clone();
        let legal_moves: Vec<u32> = move_gen.get_all_legal(&mut board, turn);
        return Self {
            board, move_gen, legal_moves
        }
    }
}
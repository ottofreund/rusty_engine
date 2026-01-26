use std::fmt::Error;

use crate::repr::{board::Board, move_gen::MoveGen, types::Color};
use crate::repr::*;


pub struct Game {
    pub board: Board,
    pub move_gen: MoveGen,
    pub legal_moves: Vec<u32>
}

impl Default for Game {
    fn default() -> Game {
        let move_gen: MoveGen = MoveGen::init();
        let mut board: Board = Board::default_board(&move_gen);
        let turn: Color = board.turn.clone();
        let legal_moves: Vec<u32> = move_gen.get_all_legal(&mut board, turn);
        return Self {
            board, move_gen, legal_moves
        }
    }
}

impl Game {
    ///Public api ease of use and safety method
    pub fn try_make_move(&mut self, init_sqr: u32, target_sqr: u32) -> Result<u32, Error> {
        let mov: Option<u32> = self.legal_moves.iter().copied().find(|mov| 
            _move::get_init(*mov) == init_sqr && _move::get_target(*mov) == target_sqr
        );
        match mov {
            Some(m) => {
                println!("Successfully moved!");
                self.make_move(m);
                return Ok(m);
            },
            None => {
                println!("Tried to make illegal move");
                return Err(Error::default())
            }
        }
    }

    ///Board state is modified and legal_moves is updated
    fn make_move(&mut self, mov: u32) {
        let is_white_turn: bool = self.board.turn.is_white();
        let is_promotion: bool = _move::is_promotion(mov);
        let from: u32 = _move::get_init(mov);
        let to: u32 = _move::get_target(mov);
        let moved_piece: usize = _move::get_moved_piece(mov) as usize;
        let is_eating: bool = _move::is_eating(mov);
        let is_castle: bool = _move::is_castle(mov);
        let own_occupation: &mut u64;
        let opponent_occupation: &mut u64;
        if is_white_turn {
            own_occupation = &mut self.board.white_occupation; opponent_occupation = &mut self.board.black_occupation;
        } else {
            own_occupation = &mut self.board.black_occupation; opponent_occupation = &mut self.board.white_occupation;
        }
        
        if is_eating { //clear eaten piece
            let mut s: usize;
            let e: usize;
            let eaten_bb: u64 = 1u64 << to;
            if is_white_turn { s = 6; e = 12; } else { s = 0; e = 6; }
            let mut found: bool = false;
            while s < e {
                if eaten_bb & self.board.pieces[s] > 0 {
                    bitboard::clear_square(&mut self.board.pieces[s], to);
                    bitboard::clear_square(opponent_occupation, to);
                    found = true;
                    break;
                }
                s += 1;
            }
            if !found {panic!("Move was eating but enemy piece wasn't in piece bb")};
        }
        bitboard::clear_square(&mut self.board.pieces[moved_piece], from);
        bitboard::clear_square(own_occupation, from);
        bitboard::set_square(own_occupation, to);
        if !is_promotion {
            bitboard::set_square(&mut self.board.pieces[moved_piece], to);
        }


        if is_promotion {
            let promotion_piece: usize = _move::get_promoted_piece(mov) as usize;
            bitboard::set_square(&mut self.board.pieces[promotion_piece], to);
        } else if is_castle {
            let rook_from: u32;
            let rook_to: u32;
            let rook_piece_idx: usize;
            if _move::is_short_castle(mov) {
                if is_white_turn {rook_from = 7; rook_to = 5; rook_piece_idx = 3;} else {rook_from = 63; rook_to = 61; rook_piece_idx = 9;}
            } else {
                if is_white_turn {rook_from = 0; rook_to = 3; rook_piece_idx = 3;} else {rook_from = 56; rook_to = 59; rook_piece_idx = 9;}
            }
            bitboard::clear_square(&mut self.board.pieces[rook_piece_idx], rook_from);
            bitboard::set_square(&mut self.board.pieces[rook_piece_idx], rook_to);
        }

        self.board.update_castling_rights(from, to, is_white_turn, moved_piece as u32);

        self.board.nof_checkers = 0;
        //1. update current mover attacked, also sets nof_checkers
        if is_white_turn {
            self.board.white_attacks = self.move_gen.compute_attacked(&mut self.board, Color::White)
        } else {
            self.board.black_attacks = self.move_gen.compute_attacked(&mut self.board, Color::Black)
        }
        
        self.board.turn = self.board.turn.opposite();

        //2. compute pinned, also finds check_block_sqrs
        let turn: Color = self.board.turn.clone();
        self.move_gen.compute_pinned(&mut self.board, turn);
        //3. compute legal moves
        self.legal_moves = self.move_gen.get_all_legal(&self.board, turn);
        
        return;
    }

    pub fn unmake_move(&self, mov: u32) {
        return;
    }

}

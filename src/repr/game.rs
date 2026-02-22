use std::fmt::Error;

use crate::repr::{board::Board, move_gen::MoveGen, types::Color};
use crate::repr::*;

use crate::utils::fen_tool::fen_to_board;


///pinned_info_stack tuple order:
///0: nof_checkers, 1: check_block_sqrs, 2: mover_pinned, 3: mover_pinned_restrictions, 4: meta_attacks
pub struct Game {
    pub board: Board,
    pub move_gen: MoveGen,
    ep_stack: Vec<Option<u32>>,
    pinned_info_stack: Vec<(u32, u64, u64, [u64; 64], u64)>,
    opponent_attacked_stack: Vec<u64>,
    legal_moves_stack: Vec<Vec<u32>>,
    pub played_moves_stack: Vec<u32>
}

impl Default for Game {
    fn default() -> Game {
        let move_gen: MoveGen = MoveGen::init();
        let mut board: Board = Board::default_board(&move_gen);
        let turn: Color = board.turn.clone();
        let legal_moves: Vec<u32> = move_gen.get_all_legal(&mut board, turn);
        let legal_moves_stack: Vec<Vec<u32>> = vec![legal_moves];
        let ep_stack: Vec<Option<u32>> = vec![None];
        let pinned_info_stack: Vec<(u32, u64, u64, [u64; 64], u64)> = vec![(0, 0, 0, [0u64 ; 64], 0)];
        let opponent_attacked: u64 = board.black_attacks;
        let opponent_attacked_stack: Vec<u64> = vec![opponent_attacked];
        let played_moves_stack: Vec<u32> = Vec::new();
        return Self {
            board, move_gen, ep_stack, pinned_info_stack, opponent_attacked_stack, legal_moves_stack, played_moves_stack
        }
    }
}

impl Game {
    
    pub fn game_with(fen: &str) -> Result<Self, &str> {
        let move_gen: MoveGen = MoveGen::init();
        let board: Board;
        match fen_to_board(fen.to_string(), &move_gen) {
            Ok(b) => board = b,
            Err(fe) => return Err("Fen error")
        }
        let legal_moves: Vec<u32> = move_gen.get_all_legal(&board, board.turn.clone());
        let legal_moves_stack: Vec<Vec<u32>> = vec![legal_moves];
        let ep_sqr: Option<u32> = board.ep_square;
        let ep_stack: Vec<Option<u32>> = vec![ep_sqr];
        let nof_checkers: u32 = board.nof_checkers;
        let check_block_sqrs: u64 = board.check_block_sqrs;
        let mover_pinned: u64;
        let mover_pinned_restrictions: [u64 ; 64];
        if board.turn.is_white() {mover_pinned = board.white_pinned; mover_pinned_restrictions = board.white_pinned_restrictions;} else {mover_pinned = board.black_pinned; mover_pinned_restrictions = board.black_pinned_restrictions;}
        let meta_attacks: u64 = board.meta_attacks;
        let pinned_info_stack: Vec<(u32, u64, u64, [u64; 64], u64)> = vec![
            (nof_checkers, check_block_sqrs, mover_pinned, mover_pinned_restrictions, meta_attacks)
            ];
        let opponent_attacked: u64;
        if board.turn.is_white() {opponent_attacked = board.black_attacks;} else {opponent_attacked = board.white_attacks;}
        let opponent_attacked_stack: Vec<u64> = vec![opponent_attacked];
        let played_moves_stack: Vec<u32> = Vec::new();
        return Ok(Self {
            board, move_gen, ep_stack, pinned_info_stack, opponent_attacked_stack, legal_moves_stack, played_moves_stack
        })
    }

    pub fn legal_moves(&self) -> &Vec<u32> {
        return self.legal_moves_stack.last().expect("legal move stack was empty, shouldn't happen");
    }

    ///Public api ease of use and safety method
    pub fn try_make_move(&mut self, init_sqr: u32, target_sqr: u32) -> Result<u32, Error> {
        let mov: Option<u32> = self.legal_moves().iter().copied().find(|mov| 
            _move::get_init(*mov) == init_sqr && _move::get_target(*mov) == target_sqr
        );
        match mov {
            Some(m) => {
                println!("Successfully moved: {}", _move::to_string(m));
                self.make_move(m);
                return Ok(m);
            },
            None => {
                println!("Tried to make illegal move");
                return Err(Error::default())
            }
        }
    }

    ///Public api ease of use and safety method
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
        let is_double_push: bool = _move::is_double_push(mov);
        let is_en_passant: bool = _move::is_en_passant(mov);
        let own_occupation: &mut u64;
        let opponent_occupation: &mut u64;
        if is_white_turn {
            own_occupation = &mut self.board.white_occupation; opponent_occupation = &mut self.board.black_occupation;
        } else {
            own_occupation = &mut self.board.black_occupation; opponent_occupation = &mut self.board.white_occupation;
        }
        
        if is_eating && !is_en_passant { //clear eaten piece, en passant has own clearing logic
            let eaten_piece: usize = _move::eaten_piece(mov).expect("Was eating but no eating piece found") as usize;
            bitboard::clear_square(&mut self.board.pieces[eaten_piece], to);
            bitboard::clear_square(opponent_occupation, to);
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
            bitboard::clear_square(own_occupation, rook_from);
            bitboard::set_square(own_occupation, rook_to);
        }

        if is_en_passant { //clear ep_square
            let opponent_pawns: &mut u64;
            let offset: i32;
            if is_white_turn {opponent_pawns = &mut self.board.pieces[6]; offset = -8} else {opponent_pawns = &mut self.board.pieces[0]; offset = 8;}
            let eating_sqr: u32 = (self.board.ep_square.expect("Made en passant, but ep_square was none") as i32 + offset) as u32;
            bitboard::clear_square(opponent_pawns, eating_sqr);
            bitboard::clear_square(opponent_occupation, eating_sqr);
        }

        if is_double_push { //update ep_square stack
            if is_white_turn {
                self.ep_stack.push(Some(to - 8));
            } else {
                self.ep_stack.push(Some(to + 8));
            }
        } else {
            self.ep_stack.push(None);
        }
        self.update_ep_sqr(); //update to board.ep_square as well
        println!("Ep square: {:?}", self.board.ep_square);

        self.board.update_castling_rights_make(from, to, is_white_turn, moved_piece as u32);

        self.board.nof_checkers = 0;
        self.board.check_block_sqrs = 0;
        
        //1. update current mover attacked, also sets nof_checkers
        //also push opponent attacked to stack
        if is_white_turn {
            self.board.white_attacks = self.move_gen.compute_attacked(&mut self.board, Color::White);
            self.opponent_attacked_stack.push(self.board.white_attacks);
        } else {
            self.board.black_attacks = self.move_gen.compute_attacked(&mut self.board, Color::Black);
            self.opponent_attacked_stack.push(self.board.black_attacks);
        }
        
        self.board.turn = self.board.turn.opposite();

        
        //2. compute pinned
        //in board updates check_block_sqrs, moved_pinned, mover_pinned_restrictions and meta_attacks
        let turn: Color = self.board.turn.clone();
        self.move_gen.compute_pinned(&mut self.board, turn);
        //3. push current pinned info to stack now after updating
        let mover_pinned: u64;
        let mover_pinned_restrictions: [u64 ; 64];
        if is_white_turn {
            mover_pinned = self.board.white_pinned; mover_pinned_restrictions = self.board.white_pinned_restrictions;
        } else {
            mover_pinned = self.board.black_pinned; mover_pinned_restrictions = self.board.black_pinned_restrictions;
        }
        self.pinned_info_stack.push(
            (self.board.nof_checkers, self.board.check_block_sqrs, mover_pinned, mover_pinned_restrictions, self.board.meta_attacks)
        );
        //4. compute legal moves
        self.legal_moves_stack.push(self.move_gen.get_all_legal(&self.board, turn));
        //5. push to played moves stack
        self.played_moves_stack.push(mov);
        return;
    }

    /// Unmakes move, resulting that the state of board and game is equivalent as to before moving.
    /// 
    /// Pop legal moves from stack
    /// Pop opponent attacked bitboard from stack (also set nof checkers)
    /// Pop pinned bitboards from stack
    /// Fetch castling rights from semaphore-like counter
    pub fn unmake_move(&mut self, mov: u32) {
        let unmaking_white_move: bool = !self.board.turn.is_white();
        let from: u32 = _move::get_init(mov);
        let to: u32 = _move::get_target(mov);
        let moved_piece: usize = _move::get_moved_piece(mov) as usize;
        let eaten_piece: Option<u32> = _move::eaten_piece(mov);
        let is_castle: bool = _move::is_castle(mov);
        let is_promotion: bool = _move::is_promotion(mov);
        let is_double_push: bool = _move::is_double_push(mov);
        let is_en_passant: bool = _move::is_en_passant(mov);
        let own_occupation: &mut u64;
        let opponent_occupation: &mut u64;
        if unmaking_white_move {
            own_occupation = &mut self.board.white_occupation; opponent_occupation = &mut self.board.black_occupation;
        } else {
            own_occupation = &mut self.board.black_occupation; opponent_occupation = &mut self.board.white_occupation;
        }

        if eaten_piece.is_some() && !is_en_passant { //return eaten piece, en passant has own returning logic
            bitboard::set_square(&mut self.board.pieces[eaten_piece.expect("Wasn't eating although checked") as usize], to);
            bitboard::set_square(opponent_occupation, to);
        }
        //moved piece updates
        bitboard::clear_square(own_occupation, to);
        bitboard::set_square(own_occupation, from);
        if !is_promotion {
            bitboard::clear_square(&mut self.board.pieces[moved_piece], to);
            bitboard::set_square(&mut self.board.pieces[moved_piece], from);
        } else {
            let promotion_piece: usize = _move::get_promoted_piece(mov) as usize;
            bitboard::clear_square(&mut self.board.pieces[promotion_piece], to);
            bitboard::set_square(&mut self.board.pieces[moved_piece], from);
        }

        if is_castle {
            let rook_from: u32;
            let rook_to: u32;
            let rook_piece_idx: usize;
            if _move::is_short_castle(mov) {
                if unmaking_white_move {rook_from = 7; rook_to = 5; rook_piece_idx = 3;} else {rook_from = 63; rook_to = 61; rook_piece_idx = 9;}
            } else {
                if unmaking_white_move {rook_from = 0; rook_to = 3; rook_piece_idx = 3;} else {rook_from = 56; rook_to = 59; rook_piece_idx = 9;}
            }
            bitboard::clear_square(&mut self.board.pieces[rook_piece_idx], rook_to);
            bitboard::set_square(&mut self.board.pieces[rook_piece_idx], rook_from);
            bitboard::clear_square(own_occupation, rook_to);
            bitboard::set_square(own_occupation, rook_from);
        }

        if is_en_passant { //return ep'd pawn to correct sqr
            let opponent_pawns: &mut u64;
            let offset: i32;
            if unmaking_white_move {opponent_pawns = &mut self.board.pieces[6]; offset = -8} else {opponent_pawns = &mut self.board.pieces[0]; offset = 8;}
            let eating_sqr: u32 = (self.board.ep_square.expect("Made en passant, but ep_square was none") as i32 + offset) as u32;
            bitboard::set_square(opponent_pawns, eating_sqr);
            bitboard::set_square(opponent_occupation, eating_sqr);
        }
        //fetch ep_square from stack
        self.ep_stack.pop();
        self.board.ep_square = self.ep_stack.last().copied().expect("ep stack was empty, shouldn't happen");

        self.board.update_castling_rights_unmake();
        //fetch pin/check info from stack and update to board
        self.pinned_info_stack.pop();
        let pinned_info: (u32, u64, u64, [u64; 64], u64) = self.pinned_info_stack.last().copied().expect("pinned info stack was empty");
        self.board.nof_checkers = pinned_info.0;
        self.board.check_block_sqrs = pinned_info.1;
        if unmaking_white_move {
            self.board.black_pinned = pinned_info.2;
            self.board.black_pinned_restrictions = pinned_info.3;
        } else {
            self.board.white_pinned = pinned_info.2;
            self.board.white_pinned_restrictions = pinned_info.3;
        }
        self.board.meta_attacks = pinned_info.4;
        //1. fetch opponent attacked from stack and update to board
        self.opponent_attacked_stack.pop();
        if unmaking_white_move {
            self.board.white_attacks = self.opponent_attacked_stack.last().copied().expect("opp attacked stack was empty");
        } else {
            self.board.black_attacks = self.opponent_attacked_stack.last().copied().expect("opp attacked stack was empty");
        }
        //update turn
        self.board.turn = self.board.turn.opposite();
        //3. pop cur legal moves from stack, so previous on top
        self.legal_moves_stack.pop().expect("legal_moves_stack was empty");
        //assert!(!self.legal_moves_stack.is_empty());
        //4. pop played moves stack
        self.played_moves_stack.pop();
        return;
    }

    fn update_ep_sqr(&mut self) {
        self.board.ep_square = self.ep_stack.last().copied().expect("ep stack was empty, shouldn't happen");
        return;
    }

}

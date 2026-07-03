use crate::{repr::{_move::NULL_MOVE, board::Board, move_gen::{AVG_BRANCH_FAC, MoveGen}, types::{B_PAWN, BLACK, BoardStateInfo, W_PAWN, WHITE}}, utils::zobrist::Zobrist};
use crate::repr::*;

use crate::utils::fen_tool::fen_to_board;

pub const MOVE_ARR_SIZE: usize = AVG_BRANCH_FAC * 40; //supports 40 ply deep search

pub struct Position {
    pub board: Board,
    pub move_arr: [u32 ; AVG_BRANCH_FAC * 40],
    pub move_arr_idx: Vec<usize>, //move_arr end idx by ply, so e.g. 0..move_arr_idx[0] is first ply idx range (idx is exclusive)
    pub board_state_info_stack: Vec<BoardStateInfo>, //previous board states, allows efficient unmaking of moves
    pub played_moves_stack: Vec<u32>,
    pub last_target: u32
}


impl Position {
    
    pub fn default(move_gen: &MoveGen, zobrist: &Zobrist) -> Position {
        let board: Board = Board::default_board(move_gen, zobrist);
        let turn: u32 = board.turn;
        let mut move_arr: [u32 ; MOVE_ARR_SIZE] = [NULL_MOVE ; MOVE_ARR_SIZE];
        let generated: usize = move_gen.generate_legal(&board, turn, &mut move_arr, 0, false, false);
        let move_arr_idx: Vec<usize> = vec![0, generated];
        let board_state_info_stack: Vec<BoardStateInfo> = vec![];
        let played_moves_stack: Vec<u32> = Vec::new();
        let last_target: u32 = NULL_MOVE;
        return Self {
            board, board_state_info_stack, played_moves_stack, move_arr, move_arr_idx, last_target
        }
    }

    pub fn position_with(fen: &str, move_gen: &MoveGen, zobrist: &Zobrist) -> Result<Self, &'static str> {
        let board: Board;
        match fen_to_board(fen.to_string(), move_gen, zobrist) {
            Ok(b) => board = b,
            Err(_) => return Err("Fen error")
        }
        let mut move_arr: [u32 ; MOVE_ARR_SIZE] = [NULL_MOVE ; MOVE_ARR_SIZE];
        let generated: usize = move_gen.generate_legal(&board, board.turn, &mut move_arr, 0, false, false);
        let move_arr_idx: Vec<usize> = vec![0, generated];

        let board_state_info_stack: Vec<BoardStateInfo> = vec![];
        let played_moves_stack: Vec<u32> = Vec::new();
        let last_target: u32 = NULL_MOVE;
        return Ok(Self {
            board, board_state_info_stack, played_moves_stack, move_arr, move_arr_idx, last_target
        })
    }

    ///Returns slice to current legal moves (ply 1)
    pub fn legal_moves(&self) -> &[u32] {
        let end: usize = self.move_arr_idx[1];
        return &self.move_arr[0..end];
    }

    ///Slice to current search moves (last ply)
    pub fn legal_search_moves(&self) -> &[u32] {
        let s: usize = self.move_arr_idx[self.move_arr_idx.len() - 2];
        let e: usize = self.move_arr_idx[self.move_arr_idx.len() - 1];
        return &self.move_arr[s..e];
    }

    ///The search bounds of current search moves in move_arr (last ply)
    ///end is exclusive
    pub fn search_move_bounds(&self) -> (usize, usize) {
        return (self.move_arr_idx[self.move_arr_idx.len() - 2], self.move_arr_idx[self.move_arr_idx.len() - 1]);
    }

    pub fn is_late_game(&self) -> bool {
        return self.board.major_minor_count <= 7;
    }

    ///Board state is modified and legal_moves is updated, assumes mov is legal
    pub fn make_move(&mut self, mov: u32, in_search: bool, in_perft_debug: bool, move_gen: &MoveGen, zobrist: &Zobrist) {
        let is_white_turn: bool = self.board.turn == WHITE;
        let is_promotion: bool = _move::is_promotion(mov);
        let from: u32 = _move::get_init(mov);
        let to: u32 = _move::get_target(mov);
        let moved_piece: usize = _move::get_moved_piece(mov) as usize;
        let is_eating: bool = _move::is_eating(mov);
        let is_castle: bool = _move::is_castle(mov);
        let is_short_castle: bool = is_castle && _move::is_short_castle(mov);
        let is_double_push: bool = _move::is_double_push(mov);
        let is_en_passant: bool = _move::is_en_passant(mov);
        let promotion_piece: Option<usize> = if is_promotion {
            Some(_move::get_promoted_piece(mov) as usize)
        } else {
            None
        };
        let eaten_piece: Option<usize> = if is_eating && !is_en_passant {
            Some(_move::eaten_piece(mov).expect("Was eating but no eating piece found") as usize)
        } else {
            None
        };
        let own_occupation: &mut u64;
        let opponent_occupation: &mut u64;
        if is_white_turn {
            own_occupation = &mut self.board.white_occupation; opponent_occupation = &mut self.board.black_occupation;
        } else {
            own_occupation = &mut self.board.black_occupation; opponent_occupation = &mut self.board.white_occupation;
        }

        let cur_board_state_info: BoardStateInfo = BoardStateInfo {
            ep_sqr: self.board.ep_square,
            nof_checkers: self.board.nof_checkers,
            check_block_sqrs: self.board.check_block_sqrs,
            mover_pinned: if is_white_turn {self.board.white_pinned} else {self.board.black_pinned},
            mover_pinned_restrictions: if is_white_turn {self.board.white_pinned_restrictions} else {self.board.black_pinned_restrictions},
            meta_attacks: self.board.meta_attacks,
            opponent_attacked: if is_white_turn {self.board.black_attacks} else {self.board.white_attacks}
        };

        self.board_state_info_stack.push(cur_board_state_info);
        
        /*
         * 
         * 1. Set and clear pieces according to move      
         * 
         */
        if is_eating && !is_en_passant { //clear eaten piece, en passant has own clearing logic
            let eaten_piece: usize = eaten_piece.expect("Was eating but no eating piece found");
            bitboard::clear_square(&mut self.board.pieces[eaten_piece], to);
            bitboard::clear_square(opponent_occupation, to);
            if eaten_piece as u32 != W_PAWN && eaten_piece as u32 != B_PAWN {
                self.board.major_minor_count -= 1;
            }
        }
        bitboard::clear_square(&mut self.board.pieces[moved_piece], from);
        bitboard::clear_square(own_occupation, from);
        bitboard::set_square(own_occupation, to);
        if !is_promotion {
            bitboard::set_square(&mut self.board.pieces[moved_piece], to);
        }

        if is_promotion {
            let promotion_piece: usize = promotion_piece.expect("Was promotion but no promotion piece found");
            bitboard::set_square(&mut self.board.pieces[promotion_piece], to);
        } else if is_castle {
            let rook_from: u32;
            let rook_to: u32;
            let rook_piece_idx: usize;
            if is_short_castle {
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
        /*
         * 
         * 2. Update rest of board state  
         * 
         */
        let lost_ep: Option<u32> = self.board.ep_square;
        if is_double_push { //update board ep_square
            if is_white_turn {
                self.board.ep_square = Some(to - 8);
            } else {
                self.board.ep_square = Some(to + 8);
            }
        } else {
            self.board.ep_square = None;
        }

        let had_ws: bool = self.board.ws(); let had_wl: bool = self.board.wl(); let had_bs: bool = self.board.bs(); let had_bl: bool = self.board.bl();
        self.board.update_castling_rights_make(from, to, is_white_turn, moved_piece as u32);
        let lost_ws: bool = had_ws && !self.board.ws();
        let lost_wl: bool = had_wl && !self.board.wl();
        let lost_bs: bool = had_bs && !self.board.bs();
        let lost_bl: bool = had_bl && !self.board.bl();

        self.board.zhash = zobrist.updated_hash_forward(
            self.board.zhash,
            from as usize,
            to as usize,
            moved_piece,
            is_white_turn,
            is_promotion,
            promotion_piece,
            is_eating,
            eaten_piece,
            is_castle,
            is_short_castle,
            is_double_push,
            is_en_passant,
            lost_ws,
            lost_wl,
            lost_bs,
            lost_bl,
            lost_ep,
        );

        self.board.nof_checkers = 0;
        self.board.check_block_sqrs = 0;
        //compute opponent attacked of next pos, also sets board.nof_checkers
        if is_white_turn {
            self.board.white_attacks = move_gen.compute_attacked(&mut self.board, WHITE);
        } else {
            self.board.black_attacks = move_gen.compute_attacked(&mut self.board, BLACK);
        }
        
        self.board.turn = self.board.turn ^ 1;
        let turn: u32 = self.board.turn;
        
        //compute pinned
        //in board updates check_block_sqrs, mover_pinned, mover_pinned_restrictions and meta_attacks
        move_gen.compute_pinned(&mut self.board, turn);
        /*
         * 
         * 2. Update self state
         * 
         */
        let move_arr_s_idx: usize;
        if in_search {
            move_arr_s_idx = self.move_arr_idx.last().copied().expect("move_arr_idx was empty");
        } else { //root shifts
            move_arr_s_idx = 0;
            self.move_arr_idx.clear();
            self.move_arr_idx.push(0); // 0 ply ends at 0 (exclusive)
        }
        let generated: usize = move_gen.generate_legal(&self.board, turn, &mut self.move_arr, move_arr_s_idx, in_search, in_perft_debug);
        self.move_arr_idx.push(move_arr_s_idx + generated);
        self.played_moves_stack.push(mov);
        self.last_target = to;
        return;
    }

    /// Unmakes move, resulting that the state of position is equivalent as to before moving.
    /// assumes mov was last made
    pub fn unmake_move(&mut self, mov: u32, zobrist: &Zobrist) {
        let unmaking_white_move: bool = self.board.turn == BLACK;
        let from: u32 = _move::get_init(mov);
        let to: u32 = _move::get_target(mov);
        let moved_piece: usize = _move::get_moved_piece(mov) as usize;
        let is_castle: bool = _move::is_castle(mov);
        let is_short_castle: bool = is_castle && _move::is_short_castle(mov);
        let is_promotion: bool = _move::is_promotion(mov);
        let promotion_piece: Option<usize> = if is_promotion {
            Some(_move::get_promoted_piece(mov) as usize)
        } else {
            None
        };
        let is_eating: bool = _move::is_eating(mov);
        let is_en_passant: bool = _move::is_en_passant(mov);
        let is_double_push: bool = _move::is_double_push(mov);
        let eaten_piece: Option<usize> = if is_eating && !is_en_passant {
            Some(_move::eaten_piece(mov).expect("Was eating but no eating piece found") as usize)
        } else {
            None
        };
        let own_occupation: &mut u64;
        let opponent_occupation: &mut u64;
        if unmaking_white_move {
            own_occupation = &mut self.board.white_occupation; opponent_occupation = &mut self.board.black_occupation;
        } else {
            own_occupation = &mut self.board.black_occupation; opponent_occupation = &mut self.board.white_occupation;
        }

        /*
         * 
         * 1. Set and clear pieces according to move      
         * 
         */

        if let Some(p) = eaten_piece { //return eaten piece, en passant has own returning logic
            bitboard::set_square(&mut self.board.pieces[p], to);
            bitboard::set_square(opponent_occupation, to);
            if p as u32 != W_PAWN && p as u32 != B_PAWN {
                self.board.major_minor_count += 1;
            }
        }
        //moved piece updates
        bitboard::clear_square(own_occupation, to);
        bitboard::set_square(own_occupation, from);
        if !is_promotion {
            bitboard::clear_square(&mut self.board.pieces[moved_piece], to);
            bitboard::set_square(&mut self.board.pieces[moved_piece], from);
        } else {
            let promotion_piece: usize = promotion_piece.expect("Was promotion but no promotion piece found");
            bitboard::clear_square(&mut self.board.pieces[promotion_piece], to);
            bitboard::set_square(&mut self.board.pieces[moved_piece], from);
        }

        if is_castle {
            let rook_from: u32;
            let rook_to: u32;
            let rook_piece_idx: usize;
            if is_short_castle {
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
            let eating_sqr: u32 = (to as i32 + offset) as u32;
            bitboard::set_square(opponent_pawns, eating_sqr);
            bitboard::set_square(opponent_occupation, eating_sqr);
        }
        /*
         * 
         * 2. Update rest of board state 
         * 
         */        
        let board_state_info: BoardStateInfo = self.board_state_info_stack.pop().expect("board state info stack was empty");

        self.board.ep_square = board_state_info.ep_sqr;

        let gained_ep: Option<u32> = self.board.ep_square;
        let had_ws: bool = self.board.ws(); let had_wl: bool = self.board.wl(); let had_bs: bool = self.board.bs(); let had_bl: bool = self.board.bl();
        self.board.update_castling_rights_unmake();
        let gained_ws: bool = !had_ws && self.board.ws();
        let gained_wl: bool = !had_wl && self.board.wl();
        let gained_bs: bool = !had_bs && self.board.bs();
        let gained_bl: bool = !had_bl && self.board.bl();
        self.board.zhash = zobrist.updated_hash_backward(
            self.board.zhash,
            from as usize,
            to as usize,
            moved_piece,
            unmaking_white_move,
            is_promotion,
            promotion_piece,
            is_eating,
            eaten_piece,
            is_castle,
            is_short_castle,
            is_double_push,
            is_en_passant,
            gained_ws,
            gained_wl,
            gained_bs,
            gained_bl,
            gained_ep,
        );

        self.board.nof_checkers = board_state_info.nof_checkers;
        self.board.check_block_sqrs = board_state_info.check_block_sqrs;
        if unmaking_white_move {
            self.board.white_pinned = board_state_info.mover_pinned;
            self.board.white_pinned_restrictions = board_state_info.mover_pinned_restrictions;
            self.board.black_attacks = board_state_info.opponent_attacked;
            
        } else {
            self.board.black_pinned = board_state_info.mover_pinned;
            self.board.black_pinned_restrictions = board_state_info.mover_pinned_restrictions;
            self.board.white_attacks = board_state_info.opponent_attacked;
        }
        self.board.meta_attacks = board_state_info.meta_attacks;

        self.board.turn = self.board.turn ^ 1;
        /*
         * 
         * 3. Update self state
         * 
         */
        self.move_arr_idx.pop().expect("move_arr_idx was empty"); //"pops legal moves"
        self.played_moves_stack.pop();
        if self.played_moves_stack.is_empty() {
            self.last_target = NULL_MOVE;
        } else {
            self.last_target = _move::get_target(self.played_moves_stack.last().copied().unwrap());
        }
        return;
    }

    pub fn in_checkmate(&self) -> bool {
        return self.board.nof_checkers > 0 && self.legal_moves().is_empty();
    }

    pub fn in_stalemate(&self) -> bool {
        return self.board.nof_checkers == 0 && self.legal_moves().is_empty();
    }

}

impl Clone for Position {
    fn clone(&self) -> Self {
        
        return Self {
            board: self.board.clone(),
            move_arr: self.move_arr.clone(),
            move_arr_idx: self.move_arr_idx.clone(),
            board_state_info_stack: self.board_state_info_stack.clone(),
            played_moves_stack: self.played_moves_stack.clone(),
            last_target: self.last_target.clone()
        }
    }
}


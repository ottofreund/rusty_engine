use crate::{repr::{_move, bitboard, board::{Board, FILES, RANKS}, move_gen::MoveGen, types::{B_BISHOP_U, B_KING_U, B_KNIGHT_U, B_PAWN_U, B_QUEEN_U, B_ROOK_U, BLACK, NOF_PIECE_TYPES_U, W_BISHOP_U, W_KING_U, W_KNIGHT_U, W_PAWN_U, W_QUEEN_U, W_ROOK_U, WHITE}}, search::eval::PIECE_MATERIAL_VALUE};

const NO_ENTRIES_IDX: usize = usize::MAX;

pub struct SeeWorker {
    attackers_white: Vec<usize>, //sqr indices of white attackers to action_sqr in descending order of piece value
    attackers_black: Vec<usize>,
    total_attackers: u64,
    piece_s_indices_white: [usize ; NOF_PIECE_TYPES_U], // piece_s_indices_white[piece_type] = index of first attacker of piece_type in attackers_white
    piece_s_indices_black: [usize ; NOF_PIECE_TYPES_U],
    lvp_white: Option<usize>, //least valuable piece type index
    lvp_black: Option<usize>,
}

impl SeeWorker {

    /// static exchange evaluation <br>
    /// Currently accepts only taking moves as initiating move <br>
    /// Returns SEE >= 0
    pub fn see_positive(&mut self, initiating_move: u32, board: &Board, move_gen: &MoveGen) -> bool {
        if _move::is_en_passant(initiating_move) { //fuck en passant
            return true;
        }

        let moved_piece: usize = _move::get_moved_piece(initiating_move) as usize;

        if moved_piece % NOF_PIECE_TYPES_U == W_KING_U {
            return true;
        }

        if _move::is_promotion(initiating_move) {
            return true;
        }

        let opponent_promotion_rank: u64 = if board.turn == WHITE { RANKS[0] } else { RANKS[7] };
        if !bitboard::contains_square(opponent_promotion_rank, _move::get_target(initiating_move)) //avoid promotion recapture edge case
           && PIECE_MATERIAL_VALUE[_move::eaten_piece(initiating_move).expect("non-take initiating move") as usize] >= PIECE_MATERIAL_VALUE[moved_piece] 
        {
            return true;
        } 
        
        let action_sqr: usize = _move::get_target(initiating_move) as usize;
        self.reset_for_new_see();
        self.add_direct_attackers(action_sqr, board, move_gen);

        fn see(
            score: i32,
            side: u32,
            action_sqr_val: i32,
            action_sqr: usize,
            mut see_occupation: u64,
            initiating_move: Option<u32>,
            board: &Board,
            move_gen: &MoveGen,
            see_worker: &mut SeeWorker
        ) -> i32 {
            
            if !see_worker.has_allowed_take(side) {
                return score;
            }

            let sqr: usize;
            let piece_type: usize;
            if let Some(init_move) = initiating_move {
                sqr = _move::get_init(init_move) as usize;
                piece_type = _move::get_moved_piece(init_move) as usize;
                see_worker.remove_selected_attacker(sqr, piece_type);
            } else {
                let sqr_piece_type = see_worker.pop_lvp(side);
                sqr = sqr_piece_type.0;
                piece_type = sqr_piece_type.1;
            }
            let piece_kind: usize = piece_type % NOF_PIECE_TYPES_U;
            let promotes: bool = piece_kind == W_PAWN_U
                && ((side == WHITE && action_sqr >> 3 == 7)
                    || (side == BLACK && action_sqr >> 3 == 0));
            // Material-only SEE assumes promotion to the most valuable piece.
            let taker_value: i32 = if promotes {
                PIECE_MATERIAL_VALUE[W_QUEEN_U] as i32
            } else {
                PIECE_MATERIAL_VALUE[piece_type] as i32
            };
            let promotion_gain: i32 = if promotes {
                PIECE_MATERIAL_VALUE[W_QUEEN_U] as i32 - PIECE_MATERIAL_VALUE[W_PAWN_U] as i32
            } else {
                0
            };
            let immediate_capture_score: i32 = score + action_sqr_val + promotion_gain;
            if immediate_capture_score < 0  { //other can stand pat next and win exchange
                return immediate_capture_score;
            }

            see_occupation ^= 1u64 << sqr;

            if piece_type != W_KNIGHT_U && piece_type != B_KNIGHT_U && piece_type != W_KING_U && piece_type != B_KING_U {
                see_worker.try_add_discovered_attacker(sqr, action_sqr, see_occupation, board, move_gen);
            }
            
            let capture_score: i32 = -see(-immediate_capture_score, side ^ 1, taker_value, action_sqr,  see_occupation, None, board, move_gen, see_worker);

            if initiating_move.is_some() { //initiating move is "forced"
                return capture_score;
            } else {
                return score.max(capture_score); //can choose between stand pat or capture
            }   
        }

        let res: i32 = see(0,
            board.turn,
            PIECE_MATERIAL_VALUE[_move::eaten_piece(initiating_move).unwrap() as usize % NOF_PIECE_TYPES_U] as i32,
            action_sqr,
            board.total_occupation(),
            Some(initiating_move),
            board,
            move_gen,
            self
        );

        return res >= 0;
    }

    /// Expects attacker Vecs are empty, index arrays are zeroed and lvps are None <br>
    /// After this attacker tables are filled with **direct attackers**, index arrays have start indices per piece type, and lvps are set
    fn add_direct_attackers(&mut self, action_sqr: usize, board: &Board, move_gen: &MoveGen) {
        //Index arrays act as freq arrays until the end of this method where they are converted to idx arrays with cumulative sum
        //kings
        let mut white_king_attackers: u64 = move_gen.attack_bbs[W_KING_U][action_sqr] & board.pieces[W_KING_U];
        let mut black_king_attackers: u64 = move_gen.attack_bbs[W_KING_U][action_sqr] & board.pieces[B_KING_U];
        while white_king_attackers != 0 {
            let king_idx: usize = bitboard::pop_lsb(&mut white_king_attackers) as usize;
            self.attackers_white.push(king_idx);
            self.total_attackers |= 1u64 << king_idx;
            self.lvp_white = Some(W_KING_U);
            self.piece_s_indices_white[W_KING_U] += 1;
        }
        while black_king_attackers != 0 {
            let king_idx: usize = bitboard::pop_lsb(&mut black_king_attackers) as usize;
            self.attackers_black.push(king_idx);
            self.total_attackers |= 1u64 << king_idx;
            self.lvp_black = Some(B_KING_U);
            self.piece_s_indices_black[W_KING_U] += 1;
        }
        //sliders
        let mut white_queen_attackers: u64 = 0;
        let mut black_queen_attackers: u64 = 0;
        let mut white_rook_attackers: u64 = 0;
        let mut black_rook_attackers: u64 = 0;
        let mut white_bishop_attackers: u64 = 0;
        let mut black_bishop_attackers: u64 = 0;
        //diag sliders
        if move_gen.attack_bbs[W_BISHOP_U][action_sqr] & 
            (board.pieces[W_BISHOP_U] | board.pieces[W_QUEEN_U] | board.pieces[B_BISHOP_U] | board.pieces[B_QUEEN_U]) > 0 
        { //quick check before magic indexing
            let diagonal_candidate_sqrs: u64 = 
                move_gen.get_sliding_for(
                    action_sqr, move_gen.get_relevant_blockers(action_sqr, board.total_occupation(), false), false
                );
            white_bishop_attackers= diagonal_candidate_sqrs & board.pieces[W_BISHOP_U];
            black_bishop_attackers = diagonal_candidate_sqrs & board.pieces[B_BISHOP_U];
            white_queen_attackers = diagonal_candidate_sqrs & board.pieces[W_QUEEN_U];
            black_queen_attackers = diagonal_candidate_sqrs & board.pieces[B_QUEEN_U];
        }
        //cardinal sliders
        if move_gen.attack_bbs[W_ROOK_U][action_sqr] & 
            (board.pieces[W_ROOK_U] | board.pieces[W_QUEEN_U] | board.pieces[B_ROOK_U] | board.pieces[B_QUEEN_U]) > 0 
        { //quick check before magic indexing
            let cardinal_candidate_sqrs: u64 = 
            move_gen.get_sliding_for(
                action_sqr, move_gen.get_relevant_blockers(action_sqr, board.total_occupation(), true), true
            );
            white_rook_attackers = cardinal_candidate_sqrs & board.pieces[W_ROOK_U];
            black_rook_attackers = cardinal_candidate_sqrs & board.pieces[B_ROOK_U];
            white_queen_attackers |= cardinal_candidate_sqrs & board.pieces[W_QUEEN_U];
            black_queen_attackers |= cardinal_candidate_sqrs & board.pieces[B_QUEEN_U];
        }

        while white_queen_attackers != 0 {
            let queen_idx: usize = bitboard::pop_lsb(&mut white_queen_attackers) as usize;
            self.attackers_white.push(queen_idx);
            self.total_attackers |= 1u64 << queen_idx;
            self.lvp_white = Some(W_QUEEN_U);
            self.piece_s_indices_white[W_QUEEN_U] += 1;
        }
        while black_queen_attackers != 0 {
            let queen_idx: usize = bitboard::pop_lsb(&mut black_queen_attackers) as usize;
            self.attackers_black.push(queen_idx);
            self.total_attackers |= 1u64 << queen_idx;
            self.lvp_black = Some(B_QUEEN_U);
            self.piece_s_indices_black[W_QUEEN_U] += 1;
        }

        while white_rook_attackers != 0 {
            let rook_idx: usize = bitboard::pop_lsb(&mut white_rook_attackers) as usize;    
            self.attackers_white.push(rook_idx);
            self.total_attackers |= 1u64 << rook_idx;
            self.lvp_white = Some(W_ROOK_U);
            self.piece_s_indices_white[W_ROOK_U] += 1;
        }
        while black_rook_attackers != 0 {
            let rook_idx: usize = bitboard::pop_lsb(&mut black_rook_attackers) as usize;
            self.attackers_black.push(rook_idx);
            self.total_attackers |= 1u64 << rook_idx;
            self.lvp_black = Some(B_ROOK_U);
            self.piece_s_indices_black[W_ROOK_U] += 1;
            
        }

        while white_bishop_attackers != 0 {
            let bishop_idx: usize = bitboard::pop_lsb(&mut white_bishop_attackers) as usize;
            self.attackers_white.push(bishop_idx);
            self.total_attackers |= 1u64 << bishop_idx;
            self.lvp_white = Some(W_BISHOP_U);
            self.piece_s_indices_white[W_BISHOP_U] += 1;
            
        }
        while black_bishop_attackers != 0 {
            let bishop_idx: usize = bitboard::pop_lsb(&mut black_bishop_attackers) as usize;
            self.attackers_black.push(bishop_idx);
            self.total_attackers |= 1u64 << bishop_idx;
            self.lvp_black = Some(B_BISHOP_U);
            self.piece_s_indices_black[W_BISHOP_U] += 1;
        }

        //knights
        let mut white_knight_attackers: u64 = move_gen.attack_bbs[W_KNIGHT_U][action_sqr] & board.pieces[W_KNIGHT_U];
        let mut black_knight_attackers: u64 = move_gen.attack_bbs[W_KNIGHT_U][action_sqr] & board.pieces[B_KNIGHT_U];
        while white_knight_attackers != 0 {
            let knight_idx: usize = bitboard::pop_lsb(&mut white_knight_attackers) as usize;
            self.attackers_white.push(knight_idx);
            self.total_attackers |= 1u64 << knight_idx;
            self.lvp_white = Some(W_KNIGHT_U);
            self.piece_s_indices_white[W_KNIGHT_U] += 1;
        }
        while black_knight_attackers != 0 {
            let knight_idx: usize = bitboard::pop_lsb(&mut black_knight_attackers) as usize;
            self.attackers_black.push(knight_idx);
            self.total_attackers |= 1u64 << knight_idx;
            self.lvp_black = Some(B_KNIGHT_U);
            self.piece_s_indices_black[W_KNIGHT_U] += 1;
        }
        
        //pawns
        // Derive source squares from the target because pawn attack tables are empty
        // for pieces placed directly on the first and eighth ranks.
        let target_sqr_bb: u64 = 1u64 << action_sqr;
        let mut white_pawn_attackers: u64 = (
            ((target_sqr_bb & !FILES[7]) >> 7) | ((target_sqr_bb & !FILES[0]) >> 9)
        ) & board.pieces[W_PAWN_U];
        let mut black_pawn_attackers: u64 = (
            ((target_sqr_bb & !FILES[7]) << 9) | ((target_sqr_bb & !FILES[0]) << 7)
        ) & board.pieces[B_PAWN_U];
        while white_pawn_attackers != 0 {
            let pawn_idx: usize = bitboard::pop_lsb(&mut white_pawn_attackers) as usize;
            self.attackers_white.push(pawn_idx);
            self.total_attackers |= 1u64 << pawn_idx;
            self.lvp_white = Some(W_PAWN_U);
            self.piece_s_indices_white[W_PAWN_U] += 1;
        } 
        while black_pawn_attackers != 0 {
            let pawn_idx: usize = bitboard::pop_lsb(&mut black_pawn_attackers) as usize;
            self.attackers_black.push(pawn_idx);
            self.total_attackers |= 1u64 << pawn_idx;
            self.lvp_black = Some(B_PAWN_U);
            self.piece_s_indices_black[W_PAWN_U] += 1;
        }

        let mut cumul_w: usize = 0;
        let mut cumul_b: usize = 0;
        for p in (0..NOF_PIECE_TYPES_U).rev() {
            let p_freq_w: usize = self.piece_s_indices_white[p];
            let p_freq_b: usize = self.piece_s_indices_black[p];
            if p_freq_w > 0 {
                self.piece_s_indices_white[p] = cumul_w;
                cumul_w += p_freq_w;
            } else {
                self.piece_s_indices_white[p] = NO_ENTRIES_IDX;
            }

            if p_freq_b > 0 {
                self.piece_s_indices_black[p] = cumul_b;
                cumul_b += p_freq_b;
            } else {
                self.piece_s_indices_black[p] = NO_ENTRIES_IDX;
            }
        }
        
    }

    /// Looks for discovered attacker after take in see. Only one can exist, can be of either color <br>
    /// Returns true if found and added, false if not
    fn try_add_discovered_attacker(&mut self, last_taker_init: usize, action_sqr: usize, see_occupation: u64, board: &Board, move_gen: &MoveGen) -> bool {
        let on_different_rank: bool = last_taker_init >> 3 != action_sqr >> 3;
        let taker_moved_diagonally_a1h8_dir: bool = 
            ((last_taker_init >> 3) + ((last_taker_init & 7) ^ 7)
               == (action_sqr >> 3) + ((action_sqr & 7) ^ 7)) 
               && on_different_rank;
        let taker_moved_diagonally_a8h1_dir: bool = 
            (last_taker_init >> 3) + (last_taker_init & 7)
              == (action_sqr >> 3) + (action_sqr & 7)
              && on_different_rank;
        let moved_diagonally: bool = taker_moved_diagonally_a1h8_dir || taker_moved_diagonally_a8h1_dir;
        

        let potential_discovered_attackers: u64;
        let mut potential_squares_bb: u64;
        if moved_diagonally {
            potential_discovered_attackers = board.pieces[W_BISHOP_U] | board.pieces[W_QUEEN_U] | board.pieces[B_BISHOP_U] | board.pieces[B_QUEEN_U];
            potential_squares_bb = move_gen.attack_bbs[W_BISHOP_U][action_sqr] & move_gen.attack_bbs[W_BISHOP_U][last_taker_init];
        } else {
            potential_discovered_attackers = board.pieces[W_ROOK_U] | board.pieces[W_QUEEN_U] | board.pieces[B_ROOK_U] | board.pieces[B_QUEEN_U];
            potential_squares_bb = move_gen.attack_bbs[W_ROOK_U][action_sqr] & move_gen.attack_bbs[W_ROOK_U][last_taker_init];
        }
        
        potential_squares_bb &= see_occupation & !self.total_attackers; //remove known attackers and use updated occupancy see_occupation


        if potential_discovered_attackers & potential_squares_bb == 0 { //cheap check before magic indexing
            return false;
        }

        let mut discovered_attackers: u64 = 
            move_gen.get_sliding_for(
                action_sqr, move_gen.get_relevant_blockers(
                    action_sqr, see_occupation, !moved_diagonally
                ), !moved_diagonally
            );
        discovered_attackers &= !self.total_attackers;
        discovered_attackers &= potential_discovered_attackers & see_occupation;
        
        if discovered_attackers > 0 {
            let disc_idx: u32 = bitboard::pop_lsb(&mut discovered_attackers);
            let owner: u32 = if bitboard::contains_square(board.white_occupation, disc_idx) { WHITE } else { BLACK };
            let disc_piece_type: u32 = board.get_piece_type_at(disc_idx, owner);
            self.add_discovered_attacker(disc_idx as usize, disc_piece_type as usize);
            return true;
        } else {
            return false;
        }


    }

    /// Returns (sqr, piece_type) of least valuable attacker for side <br>
    /// Removes from vec and attacker bb, updates index array and lvp if needed <br>
    /// Panics if no attackers for side
    fn pop_lvp(&mut self, side: u32) -> (usize, usize) {
        let attackers: &mut Vec<usize>;
        let piece_s_indices: &mut [usize ; 6];
        let lvp: &mut Option<usize>;
        if side == WHITE {
            attackers = &mut self.attackers_white;
            piece_s_indices = &mut self.piece_s_indices_white;
            lvp = &mut self.lvp_white;
        } else {
            attackers = &mut self.attackers_black;
            piece_s_indices = &mut self.piece_s_indices_black;
            lvp = &mut self.lvp_black;
        }

        let sqr: usize = attackers.pop().expect("Tried to pop lvp with empty attackers vec");
        self.total_attackers ^= 1u64 << sqr;
        let piece_type: usize = lvp.expect("Tried to pop lvp, but lvp was None") % NOF_PIECE_TYPES_U;
        
        //piece_s_indices[..piece_type] is guaranteed to be NO_ENTRIES_IDX
        if piece_s_indices[piece_type] == attackers.len() { //this type ran out?
            piece_s_indices[piece_type] = NO_ENTRIES_IDX;
            for p in (piece_type + 1)..NOF_PIECE_TYPES_U {
                if piece_s_indices[p] != NO_ENTRIES_IDX {
                    *lvp = Some(if side == WHITE {
                        p
                    } else {
                        p + NOF_PIECE_TYPES_U
                    });
                    break;
                }
            }
        }
        if attackers.len() == 0 {
            *lvp = None;
        }
        return (sqr, piece_type);
    }

    /// Removes a specified attacker while preserving the value-group ordering. <br>
    /// Updates the attacker bitboard, piece start indices, and least valuable piece.
    /// Panics if the supplied square is not an attacker of the supplied piece type.
    fn remove_selected_attacker(&mut self, sqr: usize, piece_type: usize) {
        let is_white: bool = piece_type < NOF_PIECE_TYPES_U;
        let piece_kind: usize = piece_type % NOF_PIECE_TYPES_U;
        let attackers: &mut Vec<usize>;
        let piece_s_indices: &mut [usize; NOF_PIECE_TYPES_U];
        let lvp: &mut Option<usize>;

        if is_white {
            attackers = &mut self.attackers_white;
            piece_s_indices = &mut self.piece_s_indices_white;
            lvp = &mut self.lvp_white;
        } else {
            attackers = &mut self.attackers_black;
            piece_s_indices = &mut self.piece_s_indices_black;
            lvp = &mut self.lvp_black;
        }

        let piece_start: usize = piece_s_indices[piece_kind];

        // Groups are ordered from most valuable to least valuable. The next
        // less-valuable group therefore marks this group's exclusive end.
        let piece_end: usize = (0..piece_kind)
            .rev()
            .find_map(|kind| {
                let start: usize = piece_s_indices[kind];
                (start != NO_ENTRIES_IDX).then_some(start)
            })
            .unwrap_or(attackers.len());

        let relative_idx: usize = attackers[piece_start..piece_end]
            .iter()
            .position(|attacker_sqr| *attacker_sqr == sqr)
            .expect("Selected square was not an attacker of the supplied piece type");
        let remove_idx: usize = piece_start + relative_idx;
        let piece_type_ran_out: bool = piece_end - piece_start == 1;

        attackers.remove(remove_idx);
        self.total_attackers &= !(1u64 << sqr);

        if piece_type_ran_out {
            piece_s_indices[piece_kind] = NO_ENTRIES_IDX;
        }

        // Less-valuable groups follow the removed entry and shift left by one.
        for kind in 0..piece_kind {
            if piece_s_indices[kind] != NO_ENTRIES_IDX {
                piece_s_indices[kind] -= 1;
            }
        }

        if attackers.is_empty() {
            *lvp = None;
        } else if piece_type_ran_out
            && lvp
                .as_ref()
                .map(|lvp_type| *lvp_type % NOF_PIECE_TYPES_U)
                == Some(piece_kind)
        {
            let next_lvp_kind: usize = ((piece_kind + 1)..NOF_PIECE_TYPES_U)
                .find(|kind| piece_s_indices[*kind] != NO_ENTRIES_IDX)
                .expect("Attacker list was non-empty but no next LVP existed");
            *lvp = Some(if is_white {
                next_lvp_kind
            } else {
                next_lvp_kind + NOF_PIECE_TYPES_U
            });
        }
    }
    
    /// Adds to correct spot efficiently maintaining order <br>
    /// Also updates lvp and idx array if needed
    fn add_discovered_attacker(&mut self, sqr: usize, piece_type: usize) {
        let is_white = piece_type < NOF_PIECE_TYPES_U;
        let piece_kind = piece_type % NOF_PIECE_TYPES_U;
        let attackers: &mut Vec<usize>;
        let lvp: &mut Option<usize>;
        let piece_s_indices: &mut [usize ; NOF_PIECE_TYPES_U];
        if is_white { 
            attackers = &mut self.attackers_white;
            lvp = &mut self.lvp_white;
            piece_s_indices = &mut self.piece_s_indices_white;
        } else {
            attackers = &mut self.attackers_black;
            lvp = &mut self.lvp_black;
            piece_s_indices = &mut self.piece_s_indices_black;
        };
        self.total_attackers |= 1u64 << sqr;

        if let Some(lvp_type) = *lvp {
            let lvp_kind = lvp_type % NOF_PIECE_TYPES_U;
            if lvp_kind == piece_kind {
                attackers.push(sqr);
            } else if lvp_kind > piece_kind { //new least valuable piece type
                piece_s_indices[piece_kind] = attackers.len();
                attackers.push(sqr);
                *lvp = Some(piece_type);
            } else {
                let mut insert_idx = piece_s_indices[piece_kind];
                if insert_idx == NO_ENTRIES_IDX { //steals the current spot of next least valuable piece type
                    let mut next_lvp_kind = piece_kind - 1;
                    let mut next_lvp_idx = piece_s_indices[next_lvp_kind];
                    while next_lvp_idx == NO_ENTRIES_IDX {
                        next_lvp_kind -= 1;
                        next_lvp_idx = piece_s_indices[next_lvp_kind];
                    }
                    insert_idx = next_lvp_idx;
                    piece_s_indices[piece_kind] = insert_idx;
                }

                attackers.insert(insert_idx, sqr);
                for pt in W_PAWN_U..piece_kind { //less valuable pieces shift right by 1
                    if piece_s_indices[pt] != NO_ENTRIES_IDX {
                        piece_s_indices[pt] += 1;
                    }
                }
            }
        } else {
            piece_s_indices[piece_kind] = 0;
            attackers.push(sqr);
            *lvp = Some(piece_type);
        }
    }

    fn reset_for_new_see(&mut self) {
        self.attackers_white.clear();
        self.attackers_black.clear();
        self.lvp_white = None;
        self.lvp_black = None;
        self.total_attackers = 0;
        for i in 0..NOF_PIECE_TYPES_U {
            self.piece_s_indices_white[i] = 0;
            self.piece_s_indices_black[i] = 0;
        }
    }

    fn has_allowed_take(&self, side: u32) -> bool {
        let own_lvp: Option<usize>;
        let opp_lvp: Option<usize>;
        if side == WHITE {
            own_lvp = self.lvp_white;
            opp_lvp = self.lvp_black;
        } else {
            own_lvp = self.lvp_black;
            opp_lvp = self.lvp_white;
        }
        if let Some(own_lvp_type) = own_lvp {
            if let Some(_) = opp_lvp {
                return own_lvp_type != W_KING_U && own_lvp_type != B_KING_U;
            } else {
                return true;
            }
        } else {
            return false;
        }
    }
    
}

impl Default for SeeWorker {
    fn default() -> Self {
        Self {
            attackers_white: Vec::with_capacity(8),
            attackers_black: Vec::with_capacity(8),
            total_attackers: 0,
            piece_s_indices_white: [0; NOF_PIECE_TYPES_U],
            piece_s_indices_black: [0; NOF_PIECE_TYPES_U],
            lvp_white: None,
            lvp_black: None,
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/see_tests.rs"]
mod tests;

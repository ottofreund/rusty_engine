use std::io;

use crate::{repr::{bitboard, types::*}, search::table_loader::read_table_value_file};

pub const MATE_EVAL: i32 = 1_000_000;
pub const PRUNE_EVAL: i32 = 2_000_000;
const FILE_NAMES: [&str ; 8] = [
    "pawn.txt", "knight.txt", "bishop.txt", "rook.txt", "queen.txt", "king.txt", "pawn_e.txt", "king_e.txt"
];

//pst: piece square table
pub struct Evaluator {
    pst: [Vec<i32> ; 8] // [6] == pawn late game, [7] == king late game
}

impl Default for Evaluator {
    fn default() -> Self {
        let pst: [Vec<i32>; 8] = std::array::from_fn(|i| {
            read_table_value_file(FILE_NAMES[i])
                .expect("Failed to read piece value tables!")
        });
        return Self {
            pst
        }
    }
}

impl Evaluator {
    /// simple eval based on piece square value tables
    /// mover is only required for negamax algorithm's sake
    pub fn eval(&self, pieces: [u64 ; 12], mover: u32, is_late_game: bool) -> i32 {
        let mut v: i32 = 0;
        let mover_is_white: bool = mover == WHITE;
        let o: usize;
        if mover_is_white {
            o = 0;
        } else {
            o = 6;
        }
        for p in 0usize..6 {
            let mut p_bb: u64 = pieces[p + o];
            if p_bb == 0 {
                continue;
            }
            let v_table: &Vec<i32> =  self.get_table(p, is_late_game);
            while p_bb > 0 {
                if mover_is_white {
                    v += v_table[bitboard::pop_lsb(&mut p_bb) as usize];
                } else {
                    v += v_table[63 - bitboard::pop_lsb(&mut p_bb) as usize];
                }
            }
        }
        //negamax compliant
        if mover_is_white {
            return v;
        } else {
            return v * -1;
        }
    }

    fn get_table(&self, piece: usize, is_late_game: bool) -> &Vec<i32> {
        if is_late_game  {
            let p_u32: u32 = piece as u32;
            if p_u32 == W_PAWN || p_u32 == B_PAWN {
                return &self.pst[6];
            } else if p_u32 == W_KING || p_u32 == B_KING {
                return &self.pst[7];
            } else {
                return &self.pst[piece];
            }
        } else {
            return &self.pst[piece];
        }
    }

}
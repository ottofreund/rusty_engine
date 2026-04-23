use crate::{repr::{bitboard, types::*}, search::table_loader::read_table_value_file};

pub const MATE_EVAL: i32 = 1_000_000;
pub const PRUNE_EVAL: i32 = 2_000_000;
const FILE_NAMES: [&str ; 8] = [
    "pawn_e.txt", "knight.txt", "bishop.txt", "rook.txt", "queen.txt", "king_e.txt", "pawl_l.txt", "king_l.txt"
];

//pst: piece square table
pub struct Evaluator {
    pst: [Vec<i32> ; 8] // [6] == pawn late game, [7] == king late game
}

impl Default for Evaluator {
    fn default() -> Self {
        let table_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("piece_square_tables");
        let pst: [Vec<i32>; 8] = std::array::from_fn(|i| {
            let path = table_dir.join(FILE_NAMES[i]);
            let path_str = path
                .to_str()
                .expect("piece-square table path must be valid UTF-8");
            read_table_value_file(path_str)
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
        for p in 0usize..12 {
            let mut p_bb: u64 = pieces[p];
            if p_bb == 0 {
                continue;
            }
            let v_table: &Vec<i32> = self.get_table(p % 6, is_late_game);
            if p < 6 {
                while p_bb > 0 {
                    v += v_table[bitboard::pop_lsb(&mut p_bb) as usize];
                }
            } else {
                while p_bb > 0 {
                    v -= v_table[63 - bitboard::pop_lsb(&mut p_bb) as usize];
                }
            }  
        }
        //negamax compliant
        if mover == WHITE {
            return v;
        } else {
            return v * -1;
        }
    }

    //piece without color so 0..=6
    fn get_table(&self, piece: usize, is_late_game: bool) -> &Vec<i32> {
        if is_late_game  {
            let p_u32: u32 = piece as u32;
            if p_u32 == W_PAWN {
                return &self.pst[6];
            } else if p_u32 == W_KING {
                return &self.pst[7];
            } else {
                return &self.pst[piece];
            }
        } else {
            return &self.pst[piece];
        }
    }

}
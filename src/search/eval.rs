use crate::{
    repr::{bitboard, board::FILES, types::*},
    search::table_loader::parse_table_values,
};

pub const PHASE_MULTIPLIERS: [i16; 12] = [0, 1, 1, 2, 4, 0, 0, 1, 1, 2, 4, 0]; //multiplier for each piece type, "how much weight it has on lateness of the game"
pub const MAX_LATE_GAME_PHASE: usize = 24;
const PHASE_COEFFS: [(f32, f32); MAX_LATE_GAME_PHASE + 1] = [
    (1.0, 0.0),
    (0.96, 0.04),
    (0.92, 0.08),
    (0.88, 0.12),
    (0.84, 0.16),
    (0.8, 0.2),
    (0.76, 0.24),
    (0.72, 0.28),
    (0.68, 0.32),
    (0.64, 0.36),
    (0.6, 0.4),
    (0.56, 0.44),
    (0.52, 0.48),
    (0.48, 0.52),
    (0.44, 0.56),
    (0.4, 0.6),
    (0.36, 0.64),
    (0.32, 0.68),
    (0.28, 0.72),
    (0.24, 0.76),
    (0.2, 0.8),
    (0.16, 0.84),
    (0.12, 0.88),
    (0.08, 0.92),
    (0.04, 0.96),
];

pub const MATE_EVAL: i16 = 25_000;
pub const MATE_BOUND: i16 = MATE_EVAL - 1000;
const TABLE_SOURCES: [&str; 8] = [
    include_str!("../../assets/piece_square_tables/pawn_e.txt"),
    include_str!("../../assets/piece_square_tables/knight.txt"),
    include_str!("../../assets/piece_square_tables/bishop.txt"),
    include_str!("../../assets/piece_square_tables/rook.txt"),
    include_str!("../../assets/piece_square_tables/queen.txt"),
    include_str!("../../assets/piece_square_tables/king_e.txt"),
    include_str!("../../assets/piece_square_tables/pawn_l.txt"),
    include_str!("../../assets/piece_square_tables/king_l.txt"),
];

pub const PIECE_MATERIAL_VALUE: [i16; 12] = [
    100, 320, 330, 500, 900, 20000, 100, 320, 330, 500, 900, 20000,
];
const PAWN_LATE_GAME_PST_IDX: usize = 6;
const KING_LATE_GAME_PST_IDX: usize = 7;

//Stockfish 16 inspired bonus values
const PASSED_PAWN_BONUS: [[i16; 8]; MAX_LATE_GAME_PHASE + 1] = {
    let opening = [0., 0., 15., 21., 65., 165., 280., 0.];
    let endgame = [0., 38., 40., 50., 80., 180., 270., 0.];
    let mut table = [[0; 8]; MAX_LATE_GAME_PHASE + 1];
    let mut phase = 0;
    while phase < table.len() {
        let (early, late) = PHASE_COEFFS[phase];
        let mut rank = 0;
        while rank < 8 {
            table[phase][rank] = (early * opening[rank] + late * endgame[rank]) as i16;
            rank += 1;
        }
        phase += 1;
    }
    table
};

#[inline]
fn passed_pawns(friendly: u64, enemy: u64, color: u32) -> u64 {
    let mut blockers = friendly | enemy | ((enemy & !FILES[0]) >> 1) | ((enemy & !FILES[7]) << 1);
    // Project blockers strictly behind them, leaving their own rank eligible.
    if color == WHITE {
        blockers |= blockers >> 8;
        blockers |= blockers >> 16;
        blockers |= blockers >> 32;
        friendly & !(blockers >> 8)
    } else {
        blockers |= blockers << 8;
        blockers |= blockers << 16;
        blockers |= blockers << 32;
        friendly & !(blockers << 8)
    }
}

#[inline]
fn passed_pawn_score(friendly: u64, enemy: u64, color: u32, bonuses: &[i16; 8]) -> i16 {
    if friendly == 0 {
        return 0;
    }
    let mut passers = passed_pawns(friendly, enemy, color);
    let square_flip = if color == WHITE { 0 } else { 56 };
    let mut score = 0;
    while passers != 0 {
        let relative_square = bitboard::pop_lsb(&mut passers) as usize ^ square_flip;
        score += bonuses[relative_square / 8];
    }
    score
}

//pst: piece square table
pub struct Evaluator {
    pst: [Vec<i16>; 8], // [6] == pawn late game, [7] == king late game
    pst_pawn_early: Vec<f32>,
    pst_pawn_late: Vec<f32>,
    pst_king_early: Vec<f32>,
    pst_king_late: Vec<f32>,
}

impl Default for Evaluator {
    fn default() -> Self {
        let pst = TABLE_SOURCES.map(|source| {
            parse_table_values(source).expect("embedded piece-square table must be valid")
        });
        let to_f32 = |table: &[i16]| -> Vec<f32> { table.iter().copied().map(f32::from).collect() };

        let pst_pawn_early = to_f32(&pst[W_PAWN_U]);
        let pst_pawn_late = to_f32(&pst[PAWN_LATE_GAME_PST_IDX]);
        let pst_king_early = to_f32(&pst[W_KING_U]);
        let pst_king_late = to_f32(&pst[KING_LATE_GAME_PST_IDX]);

        Self {
            pst,
            pst_pawn_early,
            pst_pawn_late,
            pst_king_early,
            pst_king_late,
        }
    }
}

impl Evaluator {
    /// mover is only required for negamax algorithm's sake <br>
    /// late_game_phase ranges from [0, MAX_LATE_GAME_PHASE]
    pub fn eval(&self, pieces: [u64; 12], mover: u32, late_game_phase: usize) -> i16 {
        let bonuses = &PASSED_PAWN_BONUS[late_game_phase];
        let mut white_v = passed_pawn_score(pieces[W_PAWN_U], pieces[B_PAWN_U], WHITE, bonuses);
        let phase_coeffs: (f32, f32) = PHASE_COEFFS[late_game_phase];

        let mut w_pawn_bb: u64 = pieces[W_PAWN_U];
        while w_pawn_bb > 0 {
            white_v += PIECE_MATERIAL_VALUE[W_PAWN_U];
            let idx: usize = bitboard::pop_lsb(&mut w_pawn_bb) as usize;
            white_v += (phase_coeffs.0 * self.pst_pawn_early[idx]
                + phase_coeffs.1 * self.pst_pawn_late[idx]) as i16;
        }

        for p in W_KNIGHT_U..=W_QUEEN_U {
            let mut p_bb: u64 = pieces[p];
            while p_bb > 0 {
                white_v += PIECE_MATERIAL_VALUE[p];
                white_v += self.pst[p][bitboard::pop_lsb(&mut p_bb) as usize];
            }
        }
        let mut w_king_bb: u64 = pieces[W_KING_U];
        while w_king_bb > 0 {
            white_v += PIECE_MATERIAL_VALUE[W_KING_U];
            let idx: usize = bitboard::pop_lsb(&mut w_king_bb) as usize;
            white_v += (phase_coeffs.0 * self.pst_king_early[idx]
                + phase_coeffs.1 * self.pst_king_late[idx]) as i16;
        }

        let mut black_v = passed_pawn_score(pieces[B_PAWN_U], pieces[W_PAWN_U], BLACK, bonuses);
        let mut b_pawn_bb: u64 = pieces[B_PAWN_U];
        while b_pawn_bb > 0 {
            black_v += PIECE_MATERIAL_VALUE[B_PAWN_U];
            let idx: usize = bitboard::pop_lsb(&mut b_pawn_bb) as usize ^ 56;
            black_v += (phase_coeffs.0 * self.pst_pawn_early[idx]
                + phase_coeffs.1 * self.pst_pawn_late[idx]) as i16;
        }

        for p in B_KNIGHT_U..=B_QUEEN_U {
            let mut p_bb: u64 = pieces[p];
            while p_bb > 0 {
                black_v += PIECE_MATERIAL_VALUE[p];
                black_v +=
                    self.pst[p - NOF_PIECE_TYPES_U][bitboard::pop_lsb(&mut p_bb) as usize ^ 56];
            }
        }
        let mut b_king_bb: u64 = pieces[B_KING_U];
        while b_king_bb > 0 {
            black_v += PIECE_MATERIAL_VALUE[B_KING_U];
            let idx: usize = bitboard::pop_lsb(&mut b_king_bb) as usize ^ 56;
            black_v += (phase_coeffs.0 * self.pst_king_early[idx]
                + phase_coeffs.1 * self.pst_king_late[idx]) as i16;
        }

        //negamax compliant
        if mover == WHITE {
            return white_v - black_v;
        } else {
            return black_v - white_v;
        }
    }
}

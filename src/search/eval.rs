use crate::{
    repr::{
        bitboard,
        board::{Board, DARK_SQUARES, FILES, RANKS},
        move_gen::MoveGen,
        types::*,
    },
    search::table_loader::parse_table_values,
};

mod kpk;

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
const BISHOP_PAIR_BONUS: i16 = 30;
const PAWN_LATE_GAME_PST_IDX: usize = 6;
const KING_LATE_GAME_PST_IDX: usize = 7;

// Normalize Stockfish-inspired bonuses using its endgame pawn value.
const PASSED_PAWN_BONUS: [[i16; 8]; MAX_LATE_GAME_PHASE + 1] = {
    let scale = PIECE_MATERIAL_VALUE[W_PAWN_U] as f32 / 208.0;
    let opening = [0., 0., 15., 21., 65., 165., 280., 0.];
    let endgame = [0., 38., 40., 50., 80., 180., 270., 0.];
    let mut table = [[0; 8]; MAX_LATE_GAME_PHASE + 1];
    let mut phase = 0;
    while phase < table.len() {
        let (early, late) = PHASE_COEFFS[phase];
        let mut rank = 0;
        while rank < 8 {
            let bonus = early * opening[rank] + late * endgame[rank];
            table[phase][rank] = (bonus * scale) as i16;
            rank += 1;
        }
        phase += 1;
    }
    table
};

#[inline]
fn passed_pawns(friendly: u64, friendly_fill: u64, enemy_fill: u64) -> u64 {
    let blockers = friendly_fill | enemy_fill | adjacent_files(enemy_fill);
    // Project blockers strictly behind them, leaving their own rank eligible.
    friendly & !(blockers >> 8)
}

#[inline]
fn passed_pawn_score(mut passers: u64, bonuses: &[i16; 8]) -> i32 {
    let mut score = 0i32;
    while passers != 0 {
        let relative_square = bitboard::pop_lsb(&mut passers) as usize;
        score += i32::from(bonuses[relative_square / 8]);
    }
    score
}

pub struct Evaluator {
    // The current material phase selects a complete set of piece-square values.
    piece_square: Box<[[[i16; 64]; NOF_PIECE_TYPES_U]; MAX_LATE_GAME_PHASE + 1]>,
}

impl Default for Evaluator {
    fn default() -> Self {
        let pst: [[i16; 64]; TABLE_SOURCES.len()] = TABLE_SOURCES.map(|source| {
            parse_table_values(source)
                .expect("embedded piece-square table must be valid")
                .try_into()
                .expect("embedded piece-square table must contain 64 values")
        });
        let piece_square = Box::new(std::array::from_fn(|phase| {
            let (early, late) = PHASE_COEFFS[phase];
            std::array::from_fn(|piece| {
                let late_table = match piece {
                    W_PAWN_U => Some(PAWN_LATE_GAME_PST_IDX),
                    W_KING_U => Some(KING_LATE_GAME_PST_IDX),
                    _ => None,
                };
                std::array::from_fn(|square| {
                    let square_score = late_table.map_or(pst[piece][square], |late_piece| {
                        (early * f32::from(pst[piece][square])
                            + late * f32::from(pst[late_piece][square]))
                            as i16
                    });
                    PIECE_MATERIAL_VALUE[piece] + square_score
                })
            })
        }));
        Self { piece_square }
    }
}

impl Evaluator {
    pub(crate) fn prepare_endgames(move_gen: &MoveGen) {
        kpk::prepare(move_gen);
    }

    /// Material, piece-square, bishop pair, and passed pawn base scores.
    pub fn eval(&self, pieces: [u64; 12], mover: u32, late_game_phase: usize) -> i16 {
        let bonuses = &PASSED_PAWN_BONUS[late_game_phase];
        let passer_score = |friendly: u64, enemy: u64| {
            passed_pawn_score(
                passed_pawns(friendly, fill_backwards(friendly), fill_backwards(enemy)),
                bonuses,
            )
        };
        let white_pawns = pieces[W_PAWN_U];
        let black_pawns = pieces[B_PAWN_U];
        let white = self.piece_square_score(&pieces[..NOF_PIECE_TYPES_U], 0, late_game_phase)
            + passer_score(white_pawns, black_pawns);
        let black = self.piece_square_score(&pieces[NOF_PIECE_TYPES_U..], 56, late_game_phase)
            + passer_score(black_pawns.swap_bytes(), white_pawns.swap_bytes());

        orient_score(white - black, mover) as i16
    }

    /// Evaluates a legal position from the side to move.
    pub fn evaluate_position(&self, board: &Board, move_gen: &MoveGen) -> i16 {
        let occupied = board.total_occupation();
        if occupied.count_ones() == 3 {
            if let Some(score) = kpk::evaluate(board, move_gen) {
                return score;
            }
        }
        let white = Side::new(board, move_gen, WHITE);
        let black = Side::new(board, move_gen, BLACK);
        let white_activity = piece_activity(&white, &black, occupied, move_gen);
        let black_activity = piece_activity(&black, &white, occupied, move_gen);
        let phase = board.late_game_phase;

        let evaluate_side = |us: &Side, them: &Side, activity: &Activity, opposing: &Activity| {
            let mut terms = us.pawns.structure;
            terms.add(activity.mobility);
            terms.middlegame += king_safety(us, them, opposing);
            terms.middlegame += threats(them, activity, opposing);
            terms.endgame += mating_king_activity(us, them);
            terms.endgame += passed_pawn_activity(us, them, activity, opposing);

            self.piece_square_score(us.pieces, us.flip, phase)
                + passed_pawn_score(us.pawns.passed, &PASSED_PAWN_BONUS[phase])
                + terms.taper(phase)
        };

        let mut score = evaluate_side(&white, &black, &white_activity, &black_activity)
            - evaluate_side(&black, &white, &black_activity, &white_activity);

        // Opposite-colored bishops can blockade pawns from squares the other bishop cannot reach.
        let bishops = white.pieces[W_BISHOP_U] | black.pieces[W_BISHOP_U];
        if board.major_minor_count == 2
            && white.pieces[W_BISHOP_U] != 0
            && black.pieces[W_BISHOP_U] != 0
            && has_bishop_pair(bishops)
        {
            let leading_pawns = if score >= 0 {
                &white.pawns
            } else {
                &black.pawns
            };
            score = score * (2 + leading_pawns.passed.count_ones().min(2) as i32) / 5;
        }

        const TEMPO: i32 = 10;
        (orient_score(score, board.turn) + TEMPO)
            .clamp(i32::from(-MATE_BOUND + 1), i32::from(MATE_BOUND - 1)) as i16
    }

    fn piece_square_score(&self, pieces: &[u64], flip: usize, phase: usize) -> i32 {
        let mut score = 0;
        for (mut bitboard, values) in pieces.iter().copied().zip(&self.piece_square[phase]) {
            while bitboard != 0 {
                let square = bitboard::pop_lsb(&mut bitboard) as usize;
                score += i32::from(values[square ^ flip]);
            }
        }

        let bishops = pieces[W_BISHOP_U];
        score + i32::from(BISHOP_PAIR_BONUS) * i32::from(has_bishop_pair(bishops))
    }
}

#[inline]
fn orient_score(score: i32, mover: u32) -> i32 {
    if mover == WHITE {
        score
    } else {
        -score
    }
}

#[inline]
fn has_bishop_pair(bishops: u64) -> bool {
    bishops & DARK_SQUARES != 0 && bishops & !DARK_SQUARES != 0
}

#[derive(Clone, Copy, Default)]
struct Score {
    middlegame: i32,
    endgame: i32,
}

impl Score {
    fn add(&mut self, score: Self) {
        self.middlegame += score.middlegame;
        self.endgame += score.endgame;
    }

    fn taper(self, phase: usize) -> i32 {
        let endgame = phase as i32;
        let middlegame = MAX_LATE_GAME_PHASE as i32 - endgame;
        (middlegame * self.middlegame + endgame * self.endgame) / MAX_LATE_GAME_PHASE as i32
    }
}

struct PawnInfo {
    attacks: u64,
    attack_span: u64,
    files: u64,
    passed: u64,
    structure: Score,
}

impl PawnInfo {
    fn new(pawns: u64, enemy: u64, color: u32) -> Self {
        // Calculate structure on a board where our pawns always move toward rank eight.
        let relative = |bb: u64| if color == WHITE { bb } else { bb.swap_bytes() };
        let friendly = relative(pawns);
        let enemy = relative(enemy);
        let friendly_fill = fill_backwards(friendly);
        let enemy_fill = fill_backwards(enemy);
        // Replicate the occupied file bits from the first rank onto every rank.
        let files = (friendly_fill & RANKS[0]) * FILES[0];
        let enemy_files = (enemy_fill & RANKS[0]) * FILES[0];
        let adjacent = adjacent_files(friendly);
        let adjacent_span = fill_forwards(adjacent);
        let attacks = adjacent << 8;
        let doubled = (friendly & (friendly_fill >> 8)).count_ones() as i32;
        let isolated = friendly & !adjacent_files(files);
        let backward =
            friendly & !isolated & !adjacent_span & ((adjacent_files(enemy) >> 16) | (enemy >> 8));
        let exposed = (isolated & !enemy_files).count_ones() as i32;
        let isolated = isolated.count_ones() as i32;
        let backward = backward.count_ones() as i32;
        let mut structure = Score {
            middlegame: -12 * doubled - 10 * isolated - 5 * exposed - 8 * backward,
            endgame: -19 * doubled - 14 * isolated - 3 * exposed - 10 * backward,
        };

        let mut connected = friendly & (attacks | adjacent);
        while connected != 0 {
            let square = bitboard::pop_lsb(&mut connected);
            let rank = (square / 8) as i32;
            structure.middlegame += 3 + rank;
            structure.endgame += 3 + rank * rank / 2;
        }

        Self {
            attacks: relative(attacks),
            attack_span: relative(adjacent_span << 8),
            files,
            passed: passed_pawns(friendly, friendly_fill, enemy_fill),
            structure,
        }
    }
}

#[inline]
fn adjacent_files(bitboard: u64) -> u64 {
    ((bitboard & !FILES[0]) >> 1) | ((bitboard & !FILES[7]) << 1)
}

#[inline]
fn fill_forwards(mut bitboard: u64) -> u64 {
    bitboard |= bitboard << 8;
    bitboard |= bitboard << 16;
    bitboard | (bitboard << 32)
}

#[inline]
fn fill_backwards(mut bitboard: u64) -> u64 {
    bitboard |= bitboard >> 8;
    bitboard |= bitboard >> 16;
    bitboard | (bitboard >> 32)
}

struct Side<'a> {
    pieces: &'a [u64; NOF_PIECE_TYPES_U],
    occupied: u64,
    pawns: PawnInfo,
    flip: usize,
    king: usize,
    king_attacks: u64,
    king_zone: u64,
}

impl<'a> Side<'a> {
    fn new(board: &'a Board, move_gen: &MoveGen, color: u32) -> Self {
        let offset = color as usize * NOF_PIECE_TYPES_U;
        let pieces = board.pieces[offset..offset + NOF_PIECE_TYPES_U]
            .try_into()
            .expect("each side has six piece types");
        let king = board.get_king_sqr_idx(color) as usize;
        let king_attacks = move_gen.attack_bbs[W_KING_U][king];
        // Center the zone one square in from the edge so a corner king still has a full ring.
        let zone_center = (king / 8).clamp(1, 6) * 8 + (king % 8).clamp(1, 6);
        let king_zone = move_gen.attack_bbs[W_KING_U][zone_center] | (1 << zone_center);
        let (occupied, flip) = if color == WHITE {
            (board.white_occupation, 0)
        } else {
            (board.black_occupation, 56)
        };
        let pawns = PawnInfo::new(
            board.pieces[offset],
            board.pieces[(color ^ 1) as usize * NOF_PIECE_TYPES_U],
            color,
        );
        Self {
            pieces,
            occupied,
            pawns,
            flip,
            king,
            king_attacks,
            king_zone,
        }
    }
}

#[derive(Default)]
struct Activity {
    mobility: Score,
    all_attacks: u64,
    minor_attacks: u64,
    pressure: i32,
    attackers: i32,
}

fn piece_activity(us: &Side, them: &Side, occupied: u64, move_gen: &MoveGen) -> Activity {
    let safe = !(us.occupied | them.pawns.attacks);
    let mut result = Activity {
        all_attacks: us.pawns.attacks | us.king_attacks,
        pressure: (us.pawns.attacks & them.king_zone).count_ones() as i32,
        ..Activity::default()
    };

    for piece in W_KNIGHT_U..=W_QUEEN_U {
        let mut pieces = us.pieces[piece];
        while pieces != 0 {
            let square = bitboard::pop_lsb(&mut pieces) as usize;
            let attacks = match piece {
                W_KNIGHT_U => move_gen.attack_bbs[W_KNIGHT_U][square],
                W_BISHOP_U => sliding_attacks(move_gen, occupied, square, false),
                W_ROOK_U => sliding_attacks(move_gen, occupied, square, true),
                _ => {
                    sliding_attacks(move_gen, occupied, square, false)
                        | sliding_attacks(move_gen, occupied, square, true)
                }
            };
            result.all_attacks |= attacks;
            let mobility = (attacks & safe).count_ones() as usize;
            let pressure = (attacks & them.king_zone & !us.occupied).count_ones() as i32;
            if pressure != 0 {
                result.attackers += 1;
                result.pressure += pressure * [3, 3, 4, 5][piece - W_KNIGHT_U];
            }

            result.mobility.add(mobility_score(piece, mobility));
            if piece <= W_BISHOP_U {
                result.minor_attacks |= attacks;
                let relative_rank = (square ^ us.flip) / 8;
                let square_mask = 1 << square;
                if (3..=5).contains(&relative_rank)
                    && us.pawns.attacks & square_mask != 0
                    && them.pawns.attack_span & square_mask == 0
                {
                    let outpost = if piece == W_KNIGHT_U { 19 } else { 9 };
                    result.mobility.add(Score {
                        middlegame: outpost,
                        endgame: outpost / 2,
                    });
                }
            } else if piece == W_ROOK_U {
                result.mobility.add(rook_activity(us, them, square));
            }
        }
    }
    result
}

#[inline]
fn sliding_attacks(move_gen: &MoveGen, occupied: u64, square: usize, rook: bool) -> u64 {
    let blockers = move_gen.get_relevant_blockers(square, occupied, rook);
    move_gen.get_sliding_for(square, blockers, rook)
}

#[inline]
fn mobility_score(piece: usize, mobility: usize) -> Score {
    let (early, late) = match piece {
        W_KNIGHT_U => (
            &[-30, -19, -10, -3, 3, 8, 13, 17, 21][..],
            &[-25, -17, -9, -2, 4, 9, 14, 18, 22][..],
        ),
        W_BISHOP_U => (
            &[-27, -16, -9, -4, 0, 4, 8, 11, 14, 17, 20, 22, 24, 26][..],
            &[-21, -14, -7, -2, 3, 7, 11, 15, 18, 21, 24, 27, 29, 31][..],
        ),
        W_ROOK_U => (
            &[-15, -9, -5, -2, 0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20][..],
            &[-33, -22, -14, -8, -3, 1, 5, 9, 13, 17, 21, 25, 29, 33, 37][..],
        ),
        _ => {
            let mobility = mobility as i32;
            return Score {
                middlegame: mobility - 5,
                endgame: 2 * mobility - 16,
            };
        }
    };
    Score {
        middlegame: early[mobility],
        endgame: late[mobility],
    }
}

fn rook_activity(us: &Side, them: &Side, square: usize) -> Score {
    let mut score = Score::default();
    if us.pawns.files & (1 << square) == 0 {
        score.middlegame += 11;
        score.endgame += 6;
        if them.pawns.files & (1 << square) == 0 {
            score.middlegame += 10;
            score.endgame += 4;
        }
    }
    let relative_rank = (square ^ us.flip) / 8;
    if relative_rank == 6 && (them.king ^ us.flip) / 8 >= 6 {
        score.middlegame += 10;
        score.endgame += 17;
    }
    score
}

fn king_safety(us: &Side, them: &Side, opposing: &Activity) -> i32 {
    let king = us.king ^ us.flip;
    let rank = king / 8;
    let mut shelter = 0;
    if rank < 3 {
        let pawns = us.pieces[W_PAWN_U];
        let pawns = if us.flip == 0 {
            pawns
        } else {
            pawns.swap_bytes()
        };
        let center = (king % 8).clamp(1, 6);
        for file in center - 1..=center + 1 {
            shelter += if pawns & FILES[file] & RANKS[rank + 1] != 0 {
                9
            } else if pawns & FILES[file] & RANKS[rank + 2] != 0 {
                3
            } else {
                -10
            };
        }
    }

    let unshielded = (opposing.all_attacks & us.king_zone & !us.pawns.attacks).count_ones() as i32;
    let queen_pressure = them.pieces[W_QUEEN_U] != 0;
    let pressure = opposing.pressure + 2 * unshielded;
    let danger = pressure * (opposing.attackers + 1) / if queen_pressure { 2 } else { 4 };
    shelter / if queen_pressure { 1 } else { 2 } - danger.min(400)
}

fn threats(them: &Side, activity: &Activity, opposing: &Activity) -> i32 {
    let minors = them.pieces[W_KNIGHT_U] | them.pieces[W_BISHOP_U];
    let majors = them.pieces[W_ROOK_U] | them.pieces[W_QUEEN_U];
    let weak = !them.pawns.attacks;
    let attacked_majors = (majors & activity.minor_attacks & weak).count_ones() as i32;
    let hanging =
        ((minors | majors) & activity.all_attacks & !opposing.all_attacks).count_ones() as i32;
    11 * attacked_majors + 13 * hanging
}

fn passed_pawn_activity(us: &Side, them: &Side, activity: &Activity, opposing: &Activity) -> i32 {
    let mut passers = us.pawns.passed;
    let mut score = 0;
    while passers != 0 {
        let square = bitboard::pop_lsb(&mut passers) as usize;
        let rank = square / 8;
        if rank < 3 || rank >= 7 {
            continue;
        }
        let advance = (square + 8) ^ us.flip;
        let advance_bit = 1 << advance;
        let enemy_distance = king_distance(them.king, advance);
        let friendly_distance = king_distance(us.king, advance);
        let unsafe_square = (them.occupied | opposing.all_attacks) & advance_bit != 0;
        let defended = activity.all_attacks & advance_bit != 0;
        let blocked = (us.occupied | them.occupied) & advance_bit != 0;
        let progress = if blocked {
            -10
        } else if !unsafe_square || defended {
            8
        } else {
            0
        };
        score += (rank as i32 - 2) * (4 * enemy_distance - 3 * friendly_distance + progress);
    }
    score
}

fn king_distance(from: usize, to: usize) -> i32 {
    (from / 8).abs_diff(to / 8).max((from % 8).abs_diff(to % 8)) as i32
}

fn mating_king_activity(us: &Side, them: &Side) -> i32 {
    if them.occupied != them.pieces[W_KING_U] || us.pieces[W_PAWN_U] != 0 {
        return 0;
    }

    let bishops = us.pieces[W_BISHOP_U];
    let heavy = us.pieces[W_ROOK_U] | us.pieces[W_QUEEN_U];
    let bishop_knight = bishops != 0 && us.pieces[W_KNIGHT_U] != 0;
    if heavy == 0 && !has_bishop_pair(bishops) && !bishop_knight {
        return 0;
    }

    let proximity = 10 * (7 - king_distance(us.king, them.king));
    let file = them.king % 8;
    let rank = them.king / 8;
    if heavy == 0 && bishop_knight && !has_bishop_pair(bishops) {
        let corners = if bishops & DARK_SQUARES != 0 {
            [0, 63]
        } else {
            [7, 56]
        };
        let distance =
            king_distance(them.king, corners[0]).min(king_distance(them.king, corners[1]));
        return proximity + 35 * (7 - distance);
    }
    let edge_distance = file.min(7 - file).min(rank.min(7 - rank));
    proximity + 40 * (3 - edge_distance as i32)
}

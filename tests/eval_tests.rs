mod common;

use std::path::PathBuf;

use common::TestEngine;
use rusty_engine::{
    repr::{
        _move,
        position::Position,
        types::{BLACK, B_KING_U, B_KNIGHT_U, B_PAWN_U, WHITE, W_KING_U, W_PAWN_U, W_QUEEN},
    },
    search::{
        eval::{Evaluator, MAX_LATE_GAME_PHASE, PIECE_MATERIAL_VALUE},
        table_loader::read_table_value_file,
    },
};

const EARLY_GAME_PHASE: usize = 0;
const MIDDLE_GAME_PHASE: usize = 12;
const LATE_GAME_PHASE: usize = MAX_LATE_GAME_PHASE;

fn tapered_value(early: i16, late: i16, coefficients: (f32, f32)) -> i16 {
    (coefficients.0 * f32::from(early) + coefficients.1 * f32::from(late)) as i16
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_table(file_name: &str) -> Vec<i16> {
    let path = repo_root()
        .join("assets")
        .join("piece_square_tables")
        .join(file_name);
    let path_str = path
        .to_str()
        .expect("table path should be valid UTF-8")
        .to_owned();
    read_table_value_file(&path_str).expect("failed to load table")
}

#[test]
fn eval_uses_piece_square_values_for_white_mover() {
    let evaluator = Evaluator::default();

    // Put on a2 (index 8) and e2 (index 12), and one white king on e1 (index 4).
    let mut pieces = [0u64; 12];
    pieces[W_PAWN_U] = (1u64 << 8) | (1u64 << 12);
    pieces[W_KING_U] = 1u64 << 4;

    let pawn_table = load_table("pawn_e.txt");
    let king_table = load_table("king_e.txt");
    let expected: i16 = 2 * PIECE_MATERIAL_VALUE[W_PAWN_U]
        + PIECE_MATERIAL_VALUE[W_KING_U]
        + pawn_table[8]
        + pawn_table[12]
        + king_table[4];

    let eval = evaluator.eval(pieces, WHITE, EARLY_GAME_PHASE);
    assert_eq!(eval, expected);
}

#[test]
fn eval_for_black_mover_is_negated_and_mirrored() {
    let evaluator = Evaluator::default();

    // One black knight on b8 (index 57). Black lookup mirrors the rank with sq ^ 56.
    let mut pieces = [0u64; 12];
    let knight_square = 57usize;
    pieces[B_KNIGHT_U] = 1u64 << knight_square;

    let knight_table = load_table("knight.txt");
    let expected = PIECE_MATERIAL_VALUE[B_KNIGHT_U] + knight_table[knight_square ^ 56];

    let eval = evaluator.eval(pieces, BLACK, EARLY_GAME_PHASE);
    assert_eq!(eval, expected);
}

#[test]
fn eval_tapers_pawn_and_king_tables_across_given_phase() {
    let evaluator = Evaluator::default();

    // Choose squares where opening and endgame tables differ.
    let mut pieces = [0u64; 12];
    pieces[W_PAWN_U] = 1u64 << 17;
    pieces[W_KING_U] = 1u64 << 20;

    let pawn_open = load_table("pawn_e.txt");
    let king_open = load_table("king_e.txt");
    let pawn_end = load_table("pawn_l.txt");
    let king_end = load_table("king_l.txt");

    let material = PIECE_MATERIAL_VALUE[W_PAWN_U] + PIECE_MATERIAL_VALUE[W_KING_U];
    let phases = [
        (EARLY_GAME_PHASE, (1.0, 0.0)),
        (MIDDLE_GAME_PHASE, (0.52, 0.48)),
        (LATE_GAME_PHASE, (0.04, 0.96)),
    ];

    for (phase, coefficients) in phases {
        let expected = material
            + tapered_value(pawn_open[17], pawn_end[17], coefficients)
            + tapered_value(king_open[20], king_end[20], coefficients);

        assert_eq!(evaluator.eval(pieces, WHITE, phase), expected);
    }
}

#[test]
fn eval_with_both_sides_pieces_is_consistent_for_each_mover() {
    let evaluator = Evaluator::default();

    // White: pawn on c3 (18), king on e1 (4)
    // Black: pawn on d6 (43), king on e8 (60)
    let mut pieces = [0u64; 12];
    pieces[W_PAWN_U] = 1u64 << 18;
    pieces[W_KING_U] = 1u64 << 4;
    pieces[B_PAWN_U] = 1u64 << 43;
    pieces[B_KING_U] = 1u64 << 60;

    let pawn_early = load_table("pawn_e.txt");
    let pawn_late = load_table("pawn_l.txt");
    let king_early = load_table("king_e.txt");
    let king_late = load_table("king_l.txt");
    let coefficients = (0.52, 0.48);

    let white_sum = PIECE_MATERIAL_VALUE[W_PAWN_U]
        + tapered_value(pawn_early[18], pawn_late[18], coefficients)
        + PIECE_MATERIAL_VALUE[W_KING_U]
        + tapered_value(king_early[4], king_late[4], coefficients);
    let black_sum = PIECE_MATERIAL_VALUE[B_PAWN_U]
        + tapered_value(pawn_early[43 ^ 56], pawn_late[43 ^ 56], coefficients)
        + PIECE_MATERIAL_VALUE[B_KING_U]
        + tapered_value(king_early[60 ^ 56], king_late[60 ^ 56], coefficients);
    let expected_white = white_sum - black_sum;
    let expected_black = -expected_white;

    let eval_from_white = evaluator.eval(pieces, WHITE, MIDDLE_GAME_PHASE);
    let eval_from_black = evaluator.eval(pieces, BLACK, MIDDLE_GAME_PHASE);

    assert_eq!(eval_from_white, expected_white);
    assert_eq!(eval_from_black, expected_black);
    assert_eq!(eval_from_white, -eval_from_black);
}

#[test]
fn board_initializes_late_game_phase_from_material() {
    let engine = TestEngine::new();

    assert_eq!(engine.default_board().late_game_phase, EARLY_GAME_PHASE);
    assert_eq!(
        engine
            .board("4k3/8/8/8/8/8/8/4K3 w - - 0 1")
            .late_game_phase,
        MAX_LATE_GAME_PHASE
    );
    assert_eq!(
        engine
            .board("4k3/8/8/8/8/8/1n6/4K2N w - - 0 1")
            .late_game_phase,
        MAX_LATE_GAME_PHASE - 2
    );

    // Synthetic promoted material above the opening phase ceiling clamps to phase zero.
    assert_eq!(
        engine
            .board("rnbqkbnr/pppppppp/8/8/8/Q7/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .late_game_phase,
        EARLY_GAME_PHASE
    );
}

#[test]
fn late_game_phase_tracks_capture_and_unmake() {
    let engine = TestEngine::new();
    let mut pos = engine.position("4k3/8/8/8/8/8/q7/R3K3 w - - 0 1");
    let initial_phase = MAX_LATE_GAME_PHASE - 6;
    assert_eq!(pos.board.late_game_phase, initial_phase);

    let capture = legal_move_matching(&pos, |mov| {
        _move::get_init(mov) == square('a', 1)
            && _move::get_target(mov) == square('a', 2)
            && _move::is_eating(mov)
    });
    engine.make_search_move(&mut pos, capture);
    assert_eq!(pos.board.late_game_phase, MAX_LATE_GAME_PHASE - 2);

    engine.unmake_move(&mut pos, capture);
    assert_eq!(pos.board.late_game_phase, initial_phase);
}

#[test]
fn late_game_phase_tracks_capture_promotion_and_unmake() {
    let engine = TestEngine::new();
    let mut pos = engine.position("4k2r/6P1/8/8/8/8/8/4K3 w - - 0 1");
    let initial_phase = MAX_LATE_GAME_PHASE - 2;
    assert_eq!(pos.board.late_game_phase, initial_phase);

    let promotion = legal_move_matching(&pos, |mov| {
        _move::get_init(mov) == square('g', 7)
            && _move::get_target(mov) == square('h', 8)
            && _move::is_eating(mov)
            && _move::is_promotion(mov)
            && _move::get_promotion_piece(mov) == W_QUEEN
    });
    engine.make_search_move(&mut pos, promotion);
    assert_eq!(pos.board.late_game_phase, MAX_LATE_GAME_PHASE - 4);

    engine.unmake_move(&mut pos, promotion);
    assert_eq!(pos.board.late_game_phase, initial_phase);
}

#[test]
fn late_game_phase_round_trip_survives_promotion_past_phase_zero() {
    let engine = TestEngine::new();
    let mut pos = engine.position("rnbqkbnr/P7/8/8/8/8/8/R1BQKB1R w - - 0 1");
    assert_eq!(pos.board.late_game_phase, 2);

    let promotion = legal_move_matching(&pos, |mov| {
        _move::get_init(mov) == square('a', 7)
            && _move::get_target(mov) == square('b', 8)
            && _move::is_eating(mov)
            && _move::is_promotion(mov)
            && _move::get_promotion_piece(mov) == W_QUEEN
    });
    engine.make_search_move(&mut pos, promotion);
    assert_eq!(pos.board.late_game_phase, EARLY_GAME_PHASE);

    engine.unmake_move(&mut pos, promotion);
    assert_eq!(pos.board.late_game_phase, 2);
}

fn legal_move_matching<F>(pos: &Position, matches: F) -> u32
where
    F: Fn(u32) -> bool,
{
    pos.legal_search_moves()
        .iter()
        .copied()
        .find(|mov| matches(*mov))
        .expect("expected matching legal move")
}

fn square(file: char, rank: u32) -> u32 {
    file as u32 - 'a' as u32 + 8 * (rank - 1)
}

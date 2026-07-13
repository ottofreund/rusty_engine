use std::path::PathBuf;

use rusty_engine::{
    repr::types::{BLACK, B_KING, B_KNIGHT, B_PAWN, WHITE, W_KING, W_PAWN},
    search::{eval::Evaluator, table_loader::read_table_value_file},
};

const PAWN_MATERIAL_VALUE: i32 = 100;
const KNIGHT_MATERIAL_VALUE: i32 = 320;
const KING_MATERIAL_VALUE: i32 = 20_000;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_table(file_name: &str) -> Vec<i32> {
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
    pieces[W_PAWN as usize] = (1u64 << 8) | (1u64 << 12);
    pieces[W_KING as usize] = 1u64 << 4;

    let pawn_table = load_table("pawn_e.txt");
    let king_table = load_table("king_e.txt");
    let expected = 2 * PAWN_MATERIAL_VALUE
        + KING_MATERIAL_VALUE
        + pawn_table[8]
        + pawn_table[12]
        + king_table[4];

    let eval = evaluator.eval(pieces, WHITE, false);
    assert_eq!(eval, expected);
}

#[test]
fn eval_for_black_mover_is_negated_and_mirrored() {
    let evaluator = Evaluator::default();

    // One black knight on b8 (index 57). Black lookup mirrors the rank with sq ^ 56.
    let mut pieces = [0u64; 12];
    let knight_square = 57usize;
    pieces[B_KNIGHT as usize] = 1u64 << knight_square;

    let knight_table = load_table("knight.txt");
    let expected = KNIGHT_MATERIAL_VALUE + knight_table[knight_square ^ 56];

    let eval = evaluator.eval(pieces, BLACK, false);
    assert_eq!(eval, expected);
}

#[test]
fn eval_uses_late_game_tables_for_pawn_and_king() {
    let evaluator = Evaluator::default();

    // Choose squares where opening and endgame tables differ.
    let mut pieces = [0u64; 12];
    pieces[W_PAWN as usize] = 1u64 << 17;
    pieces[W_KING as usize] = 1u64 << 20;

    let pawn_open = load_table("pawn_e.txt");
    let king_open = load_table("king_e.txt");
    let pawn_end = load_table("pawn_l.txt");
    let king_end = load_table("king_l.txt");

    let open_eval = evaluator.eval(pieces, WHITE, false);
    let end_eval = evaluator.eval(pieces, WHITE, true);
    let material = PAWN_MATERIAL_VALUE + KING_MATERIAL_VALUE;
    let expected_open = material + pawn_open[17] + king_open[20];
    let expected_end = material + pawn_end[17] + king_end[20];

    assert_eq!(open_eval, expected_open);
    assert_eq!(end_eval, expected_end);
    assert_ne!(open_eval, end_eval);
}

#[test]
fn eval_with_both_sides_pieces_is_consistent_for_each_mover() {
    let evaluator = Evaluator::default();

    // White: pawn on c3 (18), king on e1 (4)
    // Black: pawn on d6 (43), king on e8 (60)
    let mut pieces = [0u64; 12];
    pieces[W_PAWN as usize] = 1u64 << 18;
    pieces[W_KING as usize] = 1u64 << 4;
    pieces[B_PAWN as usize] = 1u64 << 43;
    pieces[B_KING as usize] = 1u64 << 60;

    let pawn_table = load_table("pawn_e.txt");
    let king_table = load_table("king_e.txt");

    let white_sum = PAWN_MATERIAL_VALUE + pawn_table[18] + KING_MATERIAL_VALUE + king_table[4];
    let black_sum =
        PAWN_MATERIAL_VALUE + pawn_table[43 ^ 56] + KING_MATERIAL_VALUE + king_table[60 ^ 56];
    let expected_white = white_sum - black_sum;
    let expected_black = -expected_white;

    let eval_from_white = evaluator.eval(pieces, WHITE, false);
    let eval_from_black = evaluator.eval(pieces, BLACK, false);

    assert_eq!(eval_from_white, expected_white);
    assert_eq!(eval_from_black, expected_black);
    assert_eq!(eval_from_white, -eval_from_black);
}

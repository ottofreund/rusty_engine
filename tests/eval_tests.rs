use std::path::PathBuf;

use rusty_engine::{
    repr::types::{B_KING, B_PAWN, BLACK, W_KING, W_PAWN, WHITE},
    search::{eval::Evaluator, table_loader::read_table_value_file},
};

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
    let expected = pawn_table[8] + pawn_table[12] + king_table[4];

    let eval = evaluator.eval(pieces, WHITE, false);
    assert_eq!(eval, expected);
}

#[test]
fn eval_for_black_mover_is_negated_and_mirrored() {
    let evaluator = Evaluator::default();

    // One black knight on b8 (index 57). Black lookup mirrors index with 63 - sq.
    let mut pieces = [0u64; 12];
    pieces[7] = 1u64 << 57; // B_KNIGHT index

    let knight_table = load_table("knight.txt");
    let expected = knight_table[63 - 57];

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
    let pawn_end = load_table("pawl_l.txt");
    let king_end = load_table("king_l.txt");

    let open_eval = evaluator.eval(pieces, WHITE, false);
    let end_eval = evaluator.eval(pieces, WHITE, true);
    let expected_open = pawn_open[17] + king_open[20];
    let expected_end = pawn_end[17] + king_end[20];

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

    let white_sum = pawn_table[18] + king_table[4];
    let black_sum = pawn_table[63 - 43] + king_table[63 - 60];
    let expected_white = white_sum - black_sum;
    let expected_black = -expected_white;

    let eval_from_white = evaluator.eval(pieces, WHITE, false);
    let eval_from_black = evaluator.eval(pieces, BLACK, false);

    assert_eq!(eval_from_white, expected_white);
    assert_eq!(eval_from_black, expected_black);
    assert_eq!(eval_from_white, -eval_from_black);
}

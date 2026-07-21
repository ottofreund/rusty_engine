mod common;

use common::TestEngine;
use rusty_engine::repr::move_gen::*;
use rusty_engine::repr::*;

fn generate_legal(engine: &TestEngine, fen: &str, noisy_only: bool) -> Vec<u32> {
    let board = engine.board(fen);
    let mut legal_moves = [_move::NULL_MOVE; position::MOVE_ARR_SIZE];
    let mut pseudolegal_moves = [_move::NULL_MOVE; types::MAX_PSEUDO_MOVES_IN_POS];
    let generated = engine.move_gen.generate_legal(
        &board,
        board.turn,
        &mut legal_moves,
        &mut pseudolegal_moves,
        0,
        false,
        false,
        noisy_only,
    );

    legal_moves[..generated].to_vec()
}

fn sorted_uci(moves: &[u32]) -> Vec<String> {
    let mut moves: Vec<String> = moves
        .iter()
        .map(|mov| _move::to_string(*mov, true))
        .collect();
    moves.sort();
    moves
}

fn sorted_encoded(moves: &[u32]) -> Vec<u32> {
    let mut moves = moves.to_vec();
    moves.sort_unstable();
    moves
}

#[test]
fn noisy_only_matches_captures_from_full_generation() {
    let engine = TestEngine::new();
    let fen = "4k3/8/3p1n2/4P3/2bQ4/8/8/4K3 w - - 0 1";

    let all_moves = generate_legal(&engine, fen, false);
    let captures = generate_legal(&engine, fen, true);
    let captures_from_all: Vec<u32> = all_moves
        .iter()
        .copied()
        .filter(|mov| _move::is_eating(*mov))
        .collect();

    assert!(all_moves.len() > captures.len());
    assert!(captures.iter().all(|mov| _move::is_eating(*mov)));
    assert_eq!(sorted_encoded(&captures), sorted_encoded(&captures_from_all));
    assert_eq!(
        sorted_uci(&captures),
        ["d4c4", "d4d6", "e5d6", "e5f6"]
    );
}

#[test]
fn noisy_only_includes_en_passant() {
    let engine = TestEngine::new();
    let captures = generate_legal(
        &engine,
        "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1",
        true,
    );

    assert_eq!(sorted_uci(&captures), ["e5d6"]);
    assert!(_move::is_en_passant(captures[0]));
    assert_eq!(_move::eaten_piece(captures[0]), Some(types::B_PAWN));
}

#[test]
fn noisy_only_includes_capture_and_quiet_promotions() {
    let engine = TestEngine::new();
    let noisy_moves = generate_legal(
        &engine,
        "4k2r/6P1/8/8/8/8/8/4K3 w - - 0 1",
        true,
    );

    assert!(noisy_moves.iter().all(|mov| _move::is_promotion(*mov)));
    assert_eq!(
        sorted_uci(&noisy_moves),
        [
            "g7g8b", "g7g8n", "g7g8q", "g7g8r", "g7h8b", "g7h8n", "g7h8q", "g7h8r"
        ]
    );
}

#[test]
fn naive_slide_gen_works() {
    let blockers: u64 = 18141975937152;
    println!("Blockers:\n{}", bitboard::bb_to_string(blockers));
    let mut rook_sqr: u32 = 0;
    let mut res: u64 = naive_rook_sliding(rook_sqr, blockers, true);
    println!(
        "With rook at sqr {}, got legal slides:\n{}",
        rook_sqr,
        bitboard::bb_to_string(res)
    );
    rook_sqr = 9;
    res = naive_rook_sliding(rook_sqr, blockers, true);
    println!(
        "With rook at sqr {}, got legal slides:\n{}",
        rook_sqr,
        bitboard::bb_to_string(res)
    );
    rook_sqr = 27;
    res = naive_rook_sliding(rook_sqr, blockers, true);
    println!(
        "With rook at sqr {}, got legal slides:\n{}",
        rook_sqr,
        bitboard::bb_to_string(res)
    );
}

#[test]
fn rook_sliding_bbs_are_correct() {
    let gen: MoveGen = MoveGen::init();
    let blockers: u64 = 18141975937160;
    println!("Blockers:\n{}", bitboard::bb_to_string(blockers));
    let mut rook_sqr: u32 = 0;
    let mut correct: u64 = naive_rook_sliding(rook_sqr, blockers, true);
    let mut relevant_blockers: u64 = gen.get_relevant_blockers(rook_sqr as usize, blockers, true);
    let mut precomputed: u64 = gen.get_sliding_for(rook_sqr as usize, relevant_blockers, true);
    println!("Correct:\n{}", bitboard::bb_to_string(correct));
    println!("Precomputed:\n{}", bitboard::bb_to_string(precomputed));
    assert_eq!(correct, precomputed);
    println!(
        "With rook at sqr {}, got legal slides:\n{}",
        rook_sqr,
        bitboard::bb_to_string(precomputed)
    );
    println!("NEW CASE \n");
    rook_sqr = 9;
    correct = naive_rook_sliding(rook_sqr, blockers, true);
    relevant_blockers = gen.get_relevant_blockers(rook_sqr as usize, blockers, true);
    precomputed = gen.get_sliding_for(rook_sqr as usize, relevant_blockers, true);
    assert_eq!(correct, precomputed);
    println!(
        "With rook at sqr {}, got legal slides:\n{}",
        rook_sqr,
        bitboard::bb_to_string(precomputed)
    );
    println!("NEW CASE \n");
    rook_sqr = 27;
    correct = naive_rook_sliding(rook_sqr, blockers, true);
    relevant_blockers = gen.get_relevant_blockers(rook_sqr as usize, blockers, true);
    precomputed = gen.get_sliding_for(rook_sqr as usize, relevant_blockers, true);
    assert_eq!(correct, precomputed);
    println!(
        "With rook at sqr {}, got legal slides:\n{}",
        rook_sqr,
        bitboard::bb_to_string(precomputed)
    );
    println!("NEW CASE \n");
}

#[test]
fn bishop_sliding_bbs_are_correct() {
    let gen: MoveGen = MoveGen::init();
    let blockers: u64 = 4789472650593558;
    println!("Blockers:\n{}", bitboard::bb_to_string(blockers));
    let mut bishop_sqr: u32 = 0;
    let mut correct: u64 = naive_bishop_sliding(bishop_sqr, blockers, true);
    let mut relevant_blockers: u64 =
        gen.get_relevant_blockers(bishop_sqr as usize, blockers, false);
    let mut precomputed: u64 = gen.get_sliding_for(bishop_sqr as usize, relevant_blockers, false);
    println!("Correct:\n{}", bitboard::bb_to_string(correct));
    assert_eq!(correct, precomputed);
    println!(
        "With bishop at sqr {}, got legal slides:\n{}",
        bishop_sqr,
        bitboard::bb_to_string(precomputed)
    );
    println!("NEW CASE \n");
    bishop_sqr = 49;
    correct = naive_bishop_sliding(bishop_sqr, blockers, true);
    relevant_blockers = gen.get_relevant_blockers(bishop_sqr as usize, blockers, false);
    precomputed = gen.get_sliding_for(bishop_sqr as usize, relevant_blockers, false);
    assert_eq!(correct, precomputed);
    println!(
        "With bishop at sqr {}, got legal slides:\n{}",
        bishop_sqr,
        bitboard::bb_to_string(precomputed)
    );
    println!("NEW CASE \n");
    bishop_sqr = 28;
    correct = naive_bishop_sliding(bishop_sqr, blockers, true);
    relevant_blockers = gen.get_relevant_blockers(bishop_sqr as usize, blockers, false);
    precomputed = gen.get_sliding_for(bishop_sqr as usize, relevant_blockers, false);
    assert_eq!(correct, precomputed);
    println!(
        "With bishop at sqr {}, got legal slides:\n{}",
        bishop_sqr,
        bitboard::bb_to_string(precomputed)
    );
    println!("NEW CASE \n");
}

mod common;

use common::TestEngine;
use rusty_engine::repr::move_gen::*;
use rusty_engine::repr::*;
use rusty_engine::utils::fen_tool::{board_to_fen, DEFAULT_FEN};

fn generate_legal(engine: &TestEngine, fen: &str, noisy_only: bool) -> Vec<u32> {
    let board = engine.board(fen);
    let mut legal_moves = [_move::NULL_MOVE; position::MOVE_ARR_SIZE];
    let mut pseudolegal_moves = [_move::NULL_MOVE; types::MAX_PSEUDO_MOVES_IN_POS];
    let (generated, _) = engine.move_gen.generate_legal(
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

fn compare_gives_check(
    engine: &TestEngine,
    pos: &mut position::Position,
    depth: usize,
    expected: &[(&str, u32)],
) {
    let info = engine.move_gen.check_info(&pos.board);
    let mut matched = 0;
    let (start, end) = pos.search_move_bounds();
    for i in start..end {
        let mov = pos.move_arr[i];
        let gives_check = engine.move_gen.gives_check(&pos.board, &info, mov);
        pos.make_move(mov, true, true, false, &engine.move_gen, &engine.zobrist);
        let checkers = pos.board.nof_checkers;
        assert_eq!(
            gives_check,
            checkers > 0,
            "{} leaves {}",
            _move::to_string(mov, true),
            board_to_fen(&pos.board),
        );

        if !expected.is_empty() {
            let uci = _move::to_string(mov, true);
            if let Some(&(_, count)) = expected.iter().find(|&&(name, _)| name == uci) {
                assert_eq!(checkers, count, "{uci} leaves {}", board_to_fen(&pos.board));
                matched += 1;
            }
        }
        if depth > 1 {
            compare_gives_check(engine, pos, depth - 1, &[]);
        }
        engine.unmake_move(pos, mov);
    }
    assert_eq!(
        matched,
        expected.len(),
        "missing expected moves in {}",
        board_to_fen(&pos.board)
    );
}

#[test]
fn gives_check_handles_direct_discovered_and_special_moves() {
    let engine = TestEngine::new();
    let cases: &[(&str, &[(&str, u32)])] = &[
        (
            "8/8/8/5k2/8/8/4P3/K7 w - - 0 1",
            &[("e2e4", 1), ("e2e3", 0)],
        ),
        ("k7/8/8/4p3/8/5K2/8/8 b - - 0 1", &[("e5e4", 1)]),
        ("5k2/8/6P1/8/8/8/8/K7 w - - 0 1", &[("g6g7", 1)]),
        ("7k/8/8/8/8/6p1/8/5K2 b - - 0 1", &[("g3g2", 1)]),
        (
            "4k3/8/8/8/4N3/8/8/K3R3 w - - 0 1",
            &[("e4c3", 1), ("e4f6", 2)],
        ),
        ("8/1b6/8/3r4/8/8/6K1/k7 b - - 0 1", &[("d5d4", 1)]),
        ("4k3/8/8/8/8/4P3/8/K3R3 w - - 0 1", &[("e3e4", 0)]),
        (
            "4k3/8/8/8/8/4K3/8/4R3 w - - 0 1",
            &[("e3d3", 1), ("e3e4", 0)],
        ),
        (
            "4k3/6P1/8/8/8/8/8/K7 w - - 0 1",
            &[("g7g8q", 1), ("g7g8r", 1), ("g7g8b", 0), ("g7g8n", 0)],
        ),
        (
            "k7/8/8/8/8/8/6p1/4K2R b - - 0 1",
            &[("g2h1q", 1), ("g2h1r", 1), ("g2h1b", 0), ("g2h1n", 0)],
        ),
        (
            "8/4k1P1/8/8/8/8/8/K7 w - - 0 1",
            &[("g7g8n", 1), ("g7g8q", 0)],
        ),
        ("3B4/2P5/8/k7/8/8/8/7K w - - 0 1", &[("c7c8n", 1)]),
        (
            "5k2/8/8/8/8/8/8/R3K2R w KQ - 0 1",
            &[("e1g1", 1), ("e1c1", 0)],
        ),
        (
            "r3k2r/8/8/8/8/8/8/3K4 b kq - 0 1",
            &[("e8c8", 1), ("e8g8", 0)],
        ),
        ("8/8/8/RPp4k/8/8/8/K7 w - c6 0 1", &[("b5c6", 1)]),
        ("7k/8/8/4b3/3Pp3/2K5/8/8 b - d3 0 1", &[("e4d3", 1)]),
        ("8/4k3/8/5pP1/8/8/8/K7 w - f6 0 1", &[("g5f6", 1)]),
        ("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1", &[("e5d6", 0)]),
    ];

    for &(fen, expected) in cases {
        compare_gives_check(&engine, &mut engine.position(fen), 1, expected);
    }
}

#[test]
fn gives_check_matches_make_move_in_start_position_tree() {
    let engine = TestEngine::new();
    compare_gives_check(&engine, &mut engine.position(DEFAULT_FEN), 3, &[]);
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
fn en_passant_king_safety_edge_cases() {
    let engine = TestEngine::new();
    let cases = [
        (
            "horizontal rook discovery",
            "7k/8/8/KPp4r/8/8/8/8 w - c6 0 1",
            "b5c6",
            false,
        ),
        (
            "captured pawn reveals diagonal bishop attack",
            "7k/8/2K5/3pP3/4b3/8/8/8 w - d6 0 1",
            "e5d6",
            false,
        ),
        (
            "capturing a checking pawn reveals another bishop attack",
            "7k/8/4b3/3pP3/2K5/8/8/8 w - d6 0 1",
            "e5d6",
            false,
        ),
        (
            "captured pawn reveals diagonal bishop attack for black",
            "8/8/8/4B3/3Pp3/2k5/8/7K b - d3 0 1",
            "e4d3",
            false,
        ),
        (
            "capture removes a checking pawn",
            "7k/8/8/3pP3/4K3/8/8/8 w - d6 0 1",
            "e5d6",
            true,
        ),
        (
            "destination blocks a bishop check",
            "7k/4b3/8/2KpP3/8/8/8/8 w - d6 0 1",
            "e5d6",
            true,
        ),
    ];

    for (name, fen, expected_move, expected_legal) in cases {
        let moves = sorted_uci(&generate_legal(&engine, fen, false));
        assert_eq!(
            moves.iter().any(|mov| mov == expected_move),
            expected_legal,
            "{name}: {expected_move} in {fen}"
        );
    }
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

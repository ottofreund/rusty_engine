mod common;

use common::{TestEngine, PERFT_CASES};
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

fn legal_move_by_uci(pos: &position::Position, uci: &str) -> u32 {
    pos.legal_search_moves()
        .iter()
        .copied()
        .find(|mov| _move::to_string(*mov, true) == uci)
        .unwrap_or_else(|| {
            panic!(
                "{uci} was not legal; legal moves: {:?}",
                sorted_uci(pos.legal_search_moves())
            )
        })
}

fn post_move_gives_check(engine: &TestEngine, pos: &mut position::Position, mov: u32) -> bool {
    engine.make_search_move(pos, mov);
    let gives_check = pos.board.nof_checkers > 0;
    engine.unmake_move(pos, mov);
    gives_check
}

#[test]
fn move_gives_check_handles_noisy_edge_cases() {
    let engine = TestEngine::new();
    let cases = [
        (
            "direct pawn check",
            "8/6k1/5n2/4P3/8/8/8/4K3 w - - 0 1",
            "e5f6",
            true,
        ),
        (
            "direct black pawn check",
            "7k/8/8/8/3p4/2N5/1K6/8 b - - 0 1",
            "d4c3",
            true,
        ),
        (
            "direct knight check",
            "8/7k/5p2/8/4N3/8/8/K7 w - - 0 1",
            "e4f6",
            true,
        ),
        (
            "direct bishop check",
            "6k1/7p/8/8/8/8/2B5/4K3 w - - 0 1",
            "c2h7",
            true,
        ),
        (
            "direct rook check",
            "8/p6k/8/8/8/8/8/R3K3 w - - 0 1",
            "a1a7",
            true,
        ),
        (
            "direct queen check",
            "7k/5Kp1/5Q2/8/8/8/8/8 w - - 0 1",
            "f6g7",
            true,
        ),
        (
            "knight discovers queen check",
            "4k3/8/8/2p5/4N3/8/8/K3Q3 w - - 0 1",
            "e4c5",
            true,
        ),
        (
            "bishop discovers rook check",
            "4k3/8/8/3n4/4B3/8/8/4R1K1 w - - 0 1",
            "e4d5",
            true,
        ),
        (
            "pawn discovers rook check",
            "8/8/3n4/R1P4k/8/8/8/4K3 w - - 0 1",
            "c5d6",
            true,
        ),
        (
            "en passant discovers rook check",
            "8/8/8/R1Pp3k/8/8/8/4K3 w - d6 0 1",
            "c5d6",
            true,
        ),
        (
            "black en passant discovers rook check",
            "4k3/8/8/8/K4Ppr/8/8/8 b - f3 0 1",
            "g4f3",
            true,
        ),
        (
            "white queen promotion check",
            "k7/6P1/8/8/8/8/8/4K3 w - - 0 1",
            "g7g8q",
            true,
        ),
        (
            "black knight promotion check",
            "k7/8/8/8/8/8/4K1p1/8 b - - 0 1",
            "g2g1n",
            true,
        ),
        (
            "capture destination still blocks own rook",
            "4k3/8/8/8/4n3/8/8/KB2R3 w - - 0 1",
            "b1e4",
            false,
        ),
        (
            "capture does not create a phantom slider",
            "4k3/8/8/2p5/4N3/8/8/K7 w - - 0 1",
            "e4c5",
            false,
        ),
    ];

    for (name, fen, uci, expected) in cases {
        let mut pos = engine.position(fen);
        let mov = legal_move_by_uci(&pos, uci);
        assert!(
            _move::is_eating(mov) || _move::is_promotion(mov),
            "{name}: {uci} must be in the noisy-move domain"
        );

        let predicted = engine
            .move_gen
            .move_gives_check(mov, &pos.board);
        assert_eq!(predicted, expected, "{name}: prediction for {uci} in {fen}");
        assert_eq!(
            post_move_gives_check(&engine, &mut pos, mov),
            expected,
            "{name}: post-move oracle for {uci} in {fen}"
        );
    }
}

#[test]
fn move_gives_check_matches_post_move_state_for_noisy_corpus() {
    let engine = TestEngine::new();
    let extra_fens = [
        "8/8/3n4/R1P4k/8/8/8/4K3 w - - 0 1",
        "8/8/8/R1Pp3k/8/8/8/4K3 w - d6 0 1",
        "4k3/8/8/8/K4Ppr/8/8/8 b - f3 0 1",
        "4k2r/6P1/8/8/8/8/8/4K3 w - - 0 1",
        "4k3/8/8/8/8/8/1p6/R3K3 b - - 0 1",
        "4k3/8/8/8/4n3/8/8/KB2R3 w - - 0 1",
        "4k3/8/8/2p5/4N3/8/8/K3Q3 w - - 0 1",
    ];
    let mut checked = 0;
    let mut promotions = 0;
    let mut en_passants = 0;
    let mut checking_moves = 0;

    for (name, fen) in PERFT_CASES
        .iter()
        .map(|case| (case.name, case.fen))
        .chain(extra_fens.iter().map(|fen| ("edge-case corpus", *fen)))
    {
        let mut pos = engine.position(fen);
        let noisy_moves: Vec<u32> = pos
            .legal_search_moves()
            .iter()
            .copied()
            .filter(|mov| _move::is_eating(*mov) || _move::is_promotion(*mov))
            .collect();

        for mov in noisy_moves {
            let predicted = engine.move_gen.move_gives_check(mov, &pos.board);
            let actual = post_move_gives_check(&engine, &mut pos, mov);

            assert_eq!(
                predicted,
                actual,
                "{name}: {} in {fen}",
                _move::to_string(mov, true)
            );
            checked += 1;
            promotions += _move::is_promotion(mov) as usize;
            en_passants += _move::is_en_passant(mov) as usize;
            checking_moves += actual as usize;
        }
    }

    assert!(checked >= 40, "corpus only checked {checked} noisy moves");
    assert!(
        promotions >= 16,
        "corpus only checked {promotions} promotions"
    );
    assert!(
        en_passants >= 2,
        "corpus only checked {en_passants} en passant moves"
    );
    assert!(
        checking_moves >= 10,
        "corpus only checked {checking_moves} checking moves"
    );
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

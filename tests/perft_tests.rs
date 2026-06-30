mod common;

use common::{PerftCase, TestEngine, PERFT_CASES};

#[test]
fn default_pos_perft_correct() {
    assert_perft_case(PERFT_CASES[0]);
}

#[test]
fn edge_case_perft_2() {
    assert_perft_case(PERFT_CASES[1]);
}

#[test]
fn edge_case_perft_3() {
    assert_perft_case(PERFT_CASES[2]);
}

#[test]
fn edge_case_perft_4() {
    assert_perft_case(PERFT_CASES[3]);
}

#[test]
fn edge_case_perft_5() {
    assert_perft_case(PERFT_CASES[4]);
}

#[test]
fn edge_case_perft_6() {
    assert_perft_case(PERFT_CASES[5]);
}

fn assert_perft_case(case: PerftCase) {
    let engine = TestEngine::new();
    let mut pos = engine.position(case.fen);
    assert_eq!(
        engine.perft(case.depth, &mut pos),
        case.expected,
        "{} at depth {}",
        case.name,
        case.depth
    );
}

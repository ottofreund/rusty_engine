use rusty_engine::repr::board::EDGES;
use rusty_engine::repr::move_gen::*;
use rusty_engine::repr::*;
use crate::types::Color;

#[test]
fn naive_slide_gen_works() {
    let blockers: u64 = 18141975937152;
    println!("Blockers:\n{}", bitboard::bb_to_string(blockers));
    let mut rook_sqr: u32 = 0;
    let mut res: u64 = naive_rook_sliding(rook_sqr, blockers, true);
    println!("With rook at sqr {}, got legal slides:\n{}", rook_sqr, bitboard::bb_to_string(res));
    rook_sqr = 9;
    res = naive_rook_sliding(rook_sqr, blockers, true);
    println!("With rook at sqr {}, got legal slides:\n{}", rook_sqr, bitboard::bb_to_string(res));
    rook_sqr = 27;
    res = naive_rook_sliding(rook_sqr, blockers, true);
    println!("With rook at sqr {}, got legal slides:\n{}", rook_sqr, bitboard::bb_to_string(res));
}

#[test]
fn rook_sliding_bbs_are_correct() {
    let gen: MoveGen = MoveGen::init();
    let blockers: u64 = 18141975937160;
    println!("Blockers:\n{}", bitboard::bb_to_string(blockers));
    let mut rook_sqr: u32 = 0;
    let mut correct: u64 = naive_rook_sliding(rook_sqr, blockers, true);
    let mut relevant_blockers: u64 = gen.get_relevant_blockers(rook_sqr, blockers, true, false);
    let mut precomputed: u64 = gen.get_sliding_for(rook_sqr as usize, relevant_blockers, true, false);
    println!("Correct:\n{}", bitboard::bb_to_string(correct));
    println!("Precomputed:\n{}", bitboard::bb_to_string(precomputed));
    assert_eq!(correct, precomputed);
    println!("With rook at sqr {}, got legal slides:\n{}", rook_sqr, bitboard::bb_to_string(precomputed));
    println!("NEW CASE \n");
    rook_sqr = 9;
    correct = naive_rook_sliding(rook_sqr, blockers, true);
    relevant_blockers = gen.get_relevant_blockers(rook_sqr, blockers, true, false);
    precomputed = gen.get_sliding_for(rook_sqr as usize, relevant_blockers, true, false);
    assert_eq!(correct, precomputed);
    println!("With rook at sqr {}, got legal slides:\n{}", rook_sqr, bitboard::bb_to_string(precomputed));
    println!("NEW CASE \n");
    rook_sqr = 27;
    correct = naive_rook_sliding(rook_sqr, blockers, true);
    relevant_blockers = gen.get_relevant_blockers(rook_sqr, blockers, true, false);
    precomputed = gen.get_sliding_for(rook_sqr as usize, relevant_blockers, true, false);
    assert_eq!(correct, precomputed);
    println!("With rook at sqr {}, got legal slides:\n{}", rook_sqr, bitboard::bb_to_string(precomputed));
    println!("NEW CASE \n");
}


#[test]
fn bishop_sliding_bbs_are_correct() {
    let gen: MoveGen = MoveGen::init();
    let blockers: u64 = 4789472650593558;
    println!("Blockers:\n{}", bitboard::bb_to_string(blockers));
    let mut bishop_sqr: u32 = 0;
    let mut correct: u64 = naive_bishop_sliding(bishop_sqr, blockers, true);
    let mut relevant_blockers: u64 = gen.get_relevant_blockers(bishop_sqr, blockers, false, true);
    let mut precomputed: u64 = gen.get_sliding_for(bishop_sqr as usize, relevant_blockers, false, true);
    println!("Correct:\n{}", bitboard::bb_to_string(correct));
    assert_eq!(correct, precomputed);
    println!("With bishop at sqr {}, got legal slides:\n{}", bishop_sqr, bitboard::bb_to_string(precomputed));
    println!("NEW CASE \n");
    bishop_sqr = 49;
    correct = naive_bishop_sliding(bishop_sqr, blockers, true);
    relevant_blockers = gen.get_relevant_blockers(bishop_sqr, blockers, false, true);
    precomputed = gen.get_sliding_for(bishop_sqr as usize, relevant_blockers, false, true);
    assert_eq!(correct, precomputed);
    println!("With bishop at sqr {}, got legal slides:\n{}", bishop_sqr, bitboard::bb_to_string(precomputed));
    println!("NEW CASE \n");
    bishop_sqr = 28;
    correct = naive_bishop_sliding(bishop_sqr, blockers, true);
    relevant_blockers = gen.get_relevant_blockers(bishop_sqr, blockers, false, true);
    precomputed = gen.get_sliding_for(bishop_sqr as usize, relevant_blockers, false, true);
    assert_eq!(correct, precomputed);
    println!("With bishop at sqr {}, got legal slides:\n{}", bishop_sqr, bitboard::bb_to_string(precomputed));
    println!("NEW CASE \n");
}


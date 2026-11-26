mod common;

use rusty_engine::{repr::_move, search::{
    eval::{MATE_BOUND, MATE_EVAL}, search_config::SearchMode, searcher::Searcher, tt::TranspositionTable,
}, utils::zobrist::Zobrist};

use crate::common::TestEngine;

#[test]
fn winning_mate_score_rebases_between_plies() {
    // Mate in 3, first encountered at ply 5, is mate at root ply 8.
    let score_at_old_ply = MATE_EVAL - 8;
    let stored = TranspositionTable::score_to_tt(score_at_old_ply, 5);

    assert_eq!(stored, MATE_EVAL - 3);
    assert_eq!(TranspositionTable::score_from_tt(stored, 2), MATE_EVAL - 5);
}

#[test]
fn losing_mate_score_rebases_between_plies() {
    // Mated in 4, first encountered at ply 5, is mate at root ply 9.
    let score_at_old_ply = -MATE_EVAL + 9;
    let stored = TranspositionTable::score_to_tt(score_at_old_ply, 5);

    assert_eq!(stored, -MATE_EVAL + 4);
    assert_eq!(TranspositionTable::score_from_tt(stored, 2), -MATE_EVAL + 6);
}

#[test]
fn non_mate_scores_are_not_rebased() {
    for score in [-MATE_BOUND + 1, -731, 0, 946, MATE_BOUND - 1] {
        let stored = TranspositionTable::score_to_tt(score, 17);

        assert_eq!(stored, score);
        assert_eq!(TranspositionTable::score_from_tt(stored, 4), score);
    }
}

#[test]
fn mate_bound_is_inclusive() {
    assert_eq!(
        TranspositionTable::score_to_tt(MATE_BOUND, 7),
        MATE_BOUND + 7,
    );
    assert_eq!(
        TranspositionTable::score_to_tt(-MATE_BOUND, 7),
        -MATE_BOUND - 7,
    );
}

const MULTITHREADED: bool = false;
const RULE_FIFTY_TEST_BOARD_1: &str = "6qk/8/8/8/8/8/8/K7 w - -";
const RULE_FIFTY_TEST_BOARD_2: &str = "8/8/8/8/8/2k3r1/8/2K5 w - -";
const RULE_FIFTY_TEST_BOARD_3: &str = "6rk/8/8/8/3K4/8/8/8 w - -";

#[test]
fn tt_reuse_does_not_depend_on_halfmove_clock_history() {
    let engine = TestEngine::new();
    let near_fifty = engine.position(&format!("{RULE_FIFTY_TEST_BOARD_1} 98 1")); //should be draw
    let not_near_fifty = engine.position(&format!("{RULE_FIFTY_TEST_BOARD_1} 0 1")); //should be losing for white

    let mut warmed_searcher = Searcher::from(&near_fifty, MULTITHREADED);
    configure_static_search(&mut warmed_searcher, 5, true);
    warmed_searcher.start_search(&engine.move_gen, &engine.zobrist, None);
    let near_fifty_result = root_result(&warmed_searcher);
    println!("near_fifty_result: {near_fifty_result:?}");

    warmed_searcher.import_position(&not_near_fifty, None);
    warmed_searcher.start_search(&engine.move_gen, &engine.zobrist, None);
    let not_near_fifty_result = root_result(&warmed_searcher);
    println!("not_near_fifty_result: {not_near_fifty_result:?}");

    assert!(near_fifty_result.1 == 0 && not_near_fifty_result.1 < 0);

    let near_fifty = engine.position(&format!("{RULE_FIFTY_TEST_BOARD_2} 98 1")); //should be draw
    let not_near_fifty = engine.position(&format!("{RULE_FIFTY_TEST_BOARD_2} 0 1")); //should be losing for white

    let mut warmed_searcher = Searcher::from(&near_fifty, MULTITHREADED);
    configure_static_search(&mut warmed_searcher, 5, true);
    warmed_searcher.start_search(&engine.move_gen, &engine.zobrist, None);
    let near_fifty_result = root_result(&warmed_searcher);
    println!("near_fifty_result: {near_fifty_result:?}");

    warmed_searcher.import_position(&not_near_fifty, None);
    warmed_searcher.start_search(&engine.move_gen, &engine.zobrist, None);
    let not_near_fifty_result = root_result(&warmed_searcher);
    println!("not_near_fifty_result: {not_near_fifty_result:?}");

    assert!(near_fifty_result.1 == 0 && not_near_fifty_result.1 < 0);

    let near_fifty = engine.position(&format!("{RULE_FIFTY_TEST_BOARD_3} 98 1")); //should be draw
    let not_near_fifty = engine.position(&format!("{RULE_FIFTY_TEST_BOARD_3} 0 1")); //should be losing for white

    let mut warmed_searcher = Searcher::from(&near_fifty, MULTITHREADED);
    configure_static_search(&mut warmed_searcher, 5, true);
    warmed_searcher.start_search(&engine.move_gen, &engine.zobrist, None);
    let near_fifty_result = root_result(&warmed_searcher);
    println!("near_fifty_result: {near_fifty_result:?}");

    warmed_searcher.import_position(&not_near_fifty, None);
    warmed_searcher.start_search(&engine.move_gen, &engine.zobrist, None);
    let not_near_fifty_result = root_result(&warmed_searcher);
    println!("not_near_fifty_result: {not_near_fifty_result:?}");

    assert!(near_fifty_result.1 == 0 && not_near_fifty_result.1 < 0);

}


fn configure_static_search(searcher: &mut Searcher, depth: usize, quiescence: bool) {
    searcher.search_config.search_mode = SearchMode::StaticDepth(depth);
    searcher.search_config.quiescence = quiescence;
    searcher.search_config.log_diagnostics = false;
    searcher.search_config.log_uci_diagnostics = false;
}

fn root_result(searcher: &Searcher) -> (String, i16) {
    let best_move = searcher
        .collect_best_move()
        .expect("completed depth-three search should return a move");
    let root_hash = searcher.positions[0].zhash;
    let root_score = searcher
        .tt
        .probe(Zobrist::zkey50_adjusted(root_hash, searcher.positions[0].board.half_move_clock))
        .expect("completed root search should be stored in the TT")
        .score;

    (_move::to_string(best_move, true), root_score)
}

#[test]
fn same_key_replacement_prefers_depth_then_bound() {
    use rusty_engine::search::tt::{TTEntry, TTEntryType::*};

    let cases = [
        // Existing bound, incoming bound, depth, score, generation, replace, stored generation
        (LowerBound, LowerBound, 4, 20, 3, true, 3),
        (LowerBound, LowerBound, 4, 5, 3, false, 3),
        (LowerBound, LowerBound, 4, 10, 3, false, 3),
        (UpperBound, UpperBound, 4, 5, 2, true, 3),
        (UpperBound, UpperBound, 4, 20, 3, false, 3),
        (UpperBound, UpperBound, 4, 10, 3, false, 3),
        (LowerBound, Exact, 4, 5, 3, true, 3),
        (Exact, Exact, 4, 5, 3, true, 3),
        (Exact, LowerBound, 4, 20, 3, false, 3),
        (Exact, UpperBound, 4, 5, 3, false, 3),
        (UpperBound, LowerBound, 4, 20, 3, false, 3),
        (LowerBound, UpperBound, 4, 5, 3, false, 3),
        (Exact, LowerBound, 5, 5, 3, true, 3),
        (LowerBound, Exact, 3, 20, 3, false, 2),
    ];

    let mut tt = TranspositionTable::default();
    for (index, (old_bound, new_bound, depth, score, generation, replace, expected_generation)) in
        cases.into_iter().enumerate()
    {
        tt.clear();
        let key = 123;
        let existing = TTEntry::new_packed(key, 1, 4, old_bound, 10, 2);
        let incoming = TTEntry::new_packed(key, 2, depth, new_bound, score, generation);
        tt.generation = 2;
        tt.store(existing);
        tt.generation = 3;
        tt.store(incoming);

        let stored = tt.probe(key).unwrap();
        let expected = if replace { incoming } else { existing };
        assert_eq!(stored.score, expected.score, "case {index} score");
        assert_eq!(stored.best_move, expected.best_move, "case {index} move");
        assert_eq!(
            stored.depth_and_bound_type, expected.depth_and_bound_type,
            "case {index} depth and bound"
        );
        assert_eq!(
            stored.generation, expected_generation,
            "case {index} generation"
        );
    }
}

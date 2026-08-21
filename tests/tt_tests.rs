mod common;

use common::TestEngine;
use rusty_engine::{
    repr::{_move, position::Position},
    search::{
        eval::{MATE_BOUND, MATE_EVAL},
        search_config::SearchMode,
        search_data::SearchData,
        searcher::Searcher,
        tt::TranspositionTable,
    },
};

const KINGS_ONLY_FEN: &str = "4k3/8/8/8/8/8/8/4K3 w - - 0 1";

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

#[test]
fn repetition_dependent_descendant_prevents_ancestor_tt_storage() {
    let engine = TestEngine::new();
    let mut root = engine.position(KINGS_ONLY_FEN);
    let mut history = vec![root.board.zhash];
    for mov in [
        "e1f1", "e8f8", "f1e1", "f8e8", "e1f1", "e8f8", "f1e1",
    ] {
        play_uci(&engine, &mut root, mov);
        history.push(root.board.zhash);
    }
    let root_hash = root.board.zhash;
    let root_data = SearchData::with_board_hash_history(&root, history.clone());
    assert!(!root_data.in_three_fold(&root));

    let mut repetition_child = root.clone();
    play_uci(&engine, &mut repetition_child, "f8e8");
    let mut child_history = history.clone();
    child_history.push(repetition_child.board.zhash);
    let child_data = SearchData::with_board_hash_history(&repetition_child, child_history);
    assert!(child_data.in_three_fold(&repetition_child));

    let mut independent_search = Searcher::from(&root, false);
    independent_search.search_config.search_mode = SearchMode::StaticDepth(1);
    independent_search.search_config.quiescence = false;
    independent_search.search_config.log_uci_diagnostics = false;
    independent_search.start_search(&engine.move_gen, &engine.zobrist, None);
    assert_eq!(
        independent_search
            .tt
            .probe(root_hash)
            .expect("a completed history-independent root should be stored")
            .depth(),
        1,
    );

    let mut repetition_search = Searcher::from(&root, false);
    repetition_search.import_position(&root, Some(history));
    repetition_search.search_config.search_mode = SearchMode::StaticDepth(1);
    repetition_search.search_config.quiescence = false;
    repetition_search.search_config.log_uci_diagnostics = false;
    repetition_search.start_search(&engine.move_gen, &engine.zobrist, None);

    assert!(
        repetition_search.tt.probe(root_hash).is_none(),
        "an ancestor influenced by a repetition result must not enter the TT"
    );
}

fn play_uci(engine: &TestEngine, pos: &mut Position, uci: &str) {
    let mov = pos
        .legal_moves()
        .iter()
        .copied()
        .find(|mov| _move::to_string(*mov, true) == uci)
        .unwrap_or_else(|| panic!("expected legal move {uci}"));
    pos.make_move(
        mov,
        false,
        false,
        false,
        &engine.move_gen,
        &engine.zobrist,
    );
}

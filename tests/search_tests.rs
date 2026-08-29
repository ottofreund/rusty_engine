mod common;

use crate::common::MULTITHREADED;
use common::TestEngine;
use rusty_engine::{
    repr::_move::{self, NULL_MOVE},
    repr::position::Position,
    search::{
        search_config::SearchMode,
        search_data::{get_triang_pv_ply_idx_table, TRIANG_PV_TABLE_SIZE},
        searcher::{Searcher, MAX_SEARCH_DEPTH},
    },
    utils::fen_tool::DEFAULT_FEN,
};
use std::sync::{
    atomic::{AtomicBool, Ordering::Relaxed},
    Arc,
};

const MATE_IN_ONE_FEN: &str = "7k/8/5KQ1/8/8/8/8/8 w - - 0 1";
const QUIET_MATERIAL_DISADVANTAGE_FEN: &str = "6qk/8/8/8/8/8/8/K7 w - - 0 1";
const STOP_CHECK_INTERVAL_NODES: u64 = 8192;
const CANCEL_TEST_DEPTH: usize = 9;

fn search_static_depth(engine: &TestEngine, fen: &str, depth: usize, quiescence: bool) -> Searcher {
    let pos = engine.position(fen);
    let mut searcher = Searcher::from(&pos, MULTITHREADED);
    searcher.search_config.search_mode = SearchMode::StaticDepth(depth);
    searcher.search_config.quiescence = quiescence;
    searcher.start_search(&engine.move_gen, &engine.zobrist, None);
    searcher
}

fn root_pv(searcher: &Searcher) -> Vec<u32> {
    let search_data = &searcher.search_data[0];
    let root_end = search_data.pv_ply_indices[1];

    search_data.pv[..root_end]
        .iter()
        .copied()
        .take_while(|mov| *mov != NULL_MOVE)
        .collect()
}

fn assert_legal_pv(engine: &TestEngine, start: &Position, pv: &[u32]) {
    let mut replay = start.clone();

    for mov in pv {
        assert!(
            replay.legal_search_moves().contains(mov),
            "illegal PV move {}",
            _move::to_string(*mov, true)
        );
        engine.make_search_move(&mut replay, *mov);
    }
}

#[test]
fn static_depth_quiescence_rejects_poisoned_capture() {
    let engine = TestEngine::new();
    let fen = "4k3/8/5p2/4p3/3Q4/8/8/4K3 w - - 0 1";

    let without_quiescence = search_static_depth(&engine, fen, 1, false);
    let with_quiescence = search_static_depth(&engine, fen, 1, true);
    let poisoned_capture = "d4e5";

    assert_eq!(
        _move::to_string(without_quiescence.collect_best_move().unwrap(), true),
        poisoned_capture
    );
    assert_ne!(
        _move::to_string(with_quiescence.collect_best_move().unwrap(), true),
        poisoned_capture
    );
    assert!(
        with_quiescence.search_data[0].cumul_positions_searched
            > without_quiescence.search_data[0].cumul_positions_searched
    );
}

#[test]
fn static_depth_scores_quiet_horizon_with_and_without_quiescence() {
    let engine = TestEngine::new();

    for quiescence in [false, true] {
        let searcher = search_static_depth(&engine, QUIET_MATERIAL_DISADVANTAGE_FEN, 1, quiescence);
        let root_hash = searcher.positions[0].zhash;
        let root_score = searcher
            .tt
            .probe(root_hash)
            .expect("completed root search should be stored in the TT")
            .score;

        assert!(
            root_score < 0,
            "quiet losing horizon scored {root_score} with quiescence={quiescence}"
        );
    }
}

#[test]
fn static_depth_quiescence_preserves_nominal_pv_length() {
    let engine = TestEngine::new();

    for depth in [1, 2, 3] {
        let searcher = search_static_depth(&engine, DEFAULT_FEN, depth, true);
        let pv = root_pv(&searcher);

        assert_eq!(pv.len(), depth);
        assert_eq!(searcher.search_data[0].pv_ply_indices[1], depth);
    }
}

#[test]
fn static_depth_pv_is_legal_with_and_without_quiescence() {
    let engine = TestEngine::new();

    for depth in [1, 2, 3] {
        for quiescence in [false, true] {
            let start = engine.position(DEFAULT_FEN);
            let searcher = search_static_depth(&engine, DEFAULT_FEN, depth, quiescence);
            let pv = root_pv(&searcher);

            assert_eq!(pv.len(), depth);
            assert_legal_pv(&engine, &start, &pv);
        }
    }
}

#[test]
fn static_depth_updates_quiet_history() {
    let engine = TestEngine::new();
    let searcher = search_static_depth(&engine, DEFAULT_FEN, 3, false);

    assert!(
        searcher.search_data[0]
            .history_table
            .iter()
            .any(|entry| *entry != 0),
        "expected static-depth beta cutoffs to update quiet-move history"
    );
}

#[test]
fn root_search_ages_history_once_toward_zero() {
    let engine = TestEngine::new();
    let pos = engine.position(DEFAULT_FEN);
    let mut searcher = Searcher::from(&pos, MULTITHREADED);

    searcher.search_data[0].history_table[0] = 100;
    searcher.search_data[0].history_table[1] = -100;
    searcher.search_data[0].history_table[2] = 1;
    searcher.search_data[0].history_table[3] = -1;
    searcher.search_config.search_mode = SearchMode::StaticDepth(0);

    searcher.start_search(&engine.move_gen, &engine.zobrist, None);

    assert_eq!(searcher.search_data[0].history_table[0], 75);
    assert_eq!(searcher.search_data[0].history_table[1], -75);
    assert_eq!(searcher.search_data[0].history_table[2], 0);
    assert_eq!(searcher.search_data[0].history_table[3], 0);

    searcher.start_search(&engine.move_gen, &engine.zobrist, None);

    assert_eq!(searcher.search_data[0].history_table[0], 56);
    assert_eq!(searcher.search_data[0].history_table[1], -56);
}

#[test]
fn timed_root_search_ages_history() {
    let engine = TestEngine::new();
    let pos = engine.position(DEFAULT_FEN);
    let mut searcher = Searcher::from(&pos, MULTITHREADED);
    let impossible_move_entry = 63 * 64 + 63;

    searcher.search_data[0].history_table[impossible_move_entry] = 100;
    searcher.search_config.search_mode = SearchMode::StaticTime(0);

    searcher.start_search(&engine.move_gen, &engine.zobrist, None);

    assert_eq!(
        searcher.search_data[0].history_table[impossible_move_entry],
        75
    );
}

#[test]
fn mate_in_one_terminates_static_root_pv() {
    let engine = TestEngine::new();
    let start = engine.position(MATE_IN_ONE_FEN);
    let move_source = start.clone();
    let seeded_non_mate = move_source
        .legal_search_moves()
        .iter()
        .copied()
        .find(|mov| {
            let mut child = start.clone();
            child.make_move(*mov, false, false, false, &engine.move_gen, &engine.zobrist);
            !child.legal_search_moves().is_empty()
        })
        .expect("position has a non-mating move");

    let mut searcher = Searcher::from(&start, MULTITHREADED);
    searcher.search_config.search_mode = SearchMode::StaticDepth(2);
    searcher.search_config.quiescence = false;
    searcher.search_data[0].pv[0] = seeded_non_mate;
    searcher.start_search(&engine.move_gen, &engine.zobrist, None);
    let pv = root_pv(&searcher);

    assert_eq!(pv.len(), 1);
    assert_ne!(pv[0], seeded_non_mate);
    assert_eq!(searcher.search_data[0].pv[1], NULL_MOVE);
    assert_legal_pv(&engine, &start, &pv);

    let mut result = start;
    result.make_move(
        pv[0],
        false,
        false,
        false,
        &engine.move_gen,
        &engine.zobrist,
    );
    assert!(
        result.in_checkmate(),
        "expected mate-in-one, got {}",
        _move::to_string(pv[0], true)
    );
}

#[test]
fn consecutive_static_search_syncs_exact_pv_tail() {
    let engine = TestEngine::new();
    let mut pos = engine.position(DEFAULT_FEN);
    let mut searcher = Searcher::from(&pos, MULTITHREADED);
    searcher.search_config.search_mode = SearchMode::StaticDepth(3);
    searcher.search_config.quiescence = false;

    for _ in 0..2 {
        searcher.start_search(&engine.move_gen, &engine.zobrist, None);
        let before = root_pv(&searcher);

        assert_eq!(before.len(), 3);
        assert_legal_pv(&engine, &pos, &before);

        let best_move = before[0];
        pos.make_move(
            best_move,
            false,
            false,
            false,
            &engine.move_gen,
            &engine.zobrist,
        );
        searcher.sync_new_move(&pos, Some(best_move));

        assert_eq!(root_pv(&searcher), before[1..]);
        assert_eq!(searcher.collect_best_move(), before.get(1).copied());
    }
}

#[test]
fn timed_search_gracefully_handles_abort_and_uses_incomplete_search_when_possible() {
    let engine = TestEngine::new();
    let start = engine.position(DEFAULT_FEN);
    let mut searcher = Searcher::from(&start, MULTITHREADED);
    searcher.search_config.quiescence = false;
    searcher.search_config.search_mode = SearchMode::StaticDepth(3);
    searcher.start_search(&engine.move_gen, &engine.zobrist, None);

    let completed = root_pv(&searcher);
    assert_eq!(completed.len(), 3);
    assert_legal_pv(&engine, &start, &completed);

    searcher.search_config.search_mode = SearchMode::StaticTime(0);
    searcher.start_search(&engine.move_gen, &engine.zobrist, None);

    assert_legal_pv(&engine, &start, &root_pv(&searcher));
    assert_legal_pv(&engine, &start, &completed);
}

#[test]
fn static_depth_observes_preset_kill_switch_at_poll_interval() {
    let engine = TestEngine::new();
    let start = engine.position(DEFAULT_FEN);
    let mut killed = Searcher::from(&start, MULTITHREADED);
    killed.search_config.search_mode = SearchMode::StaticDepth(CANCEL_TEST_DEPTH);
    killed.search_config.quiescence = false;
    killed.search_config.log_diagnostics = false;
    killed.search_config.log_uci_diagnostics = true;
    let kill_switch = Arc::new(AtomicBool::new(true));

    killed.start_search(
        &engine.move_gen,
        &engine.zobrist,
        Some(kill_switch.clone()),
    );

    let interrupted_depth = killed.search_data[0].pv_ply_indices[1];
    assert!((1..=CANCEL_TEST_DEPTH).contains(&interrupted_depth));

    if interrupted_depth == CANCEL_TEST_DEPTH {
        panic!("Search is too fast to test kill switch; need to increase CANCEL_TEST_DEPTH");
    }

    let mut completed = Searcher::from(&start, MULTITHREADED);
    completed.search_config.search_mode = SearchMode::StaticDepth(interrupted_depth - 1);
    completed.search_config.quiescence = false;
    completed.search_config.log_diagnostics = false;
    completed.search_config.log_uci_diagnostics = false;
    completed.start_search(&engine.move_gen, &engine.zobrist, None);

    let killed_data = &killed.search_data[0];
    let completed_nodes = completed.search_data[0].cumul_positions_searched;
    assert_eq!(
        killed_data
            .cumul_positions_searched
            .checked_sub(completed_nodes),
        Some(STOP_CHECK_INTERVAL_NODES)
    );
    assert!(kill_switch.load(Relaxed));
    assert_eq!(killed_data.positions_searched, 0);
    assert_eq!(killed_data.ab_cutoffs, 0);
    assert_eq!(killed_data.stand_pat_cutoffs, 0);
    assert_eq!(killed_data.sel_depth, 0);
    assert_eq!(killed_data.board_hash_history, vec![start.zhash]);
    assert!(
        killed.positions[0]
            .board
            .eq(&start.board, &engine.move_gen)
    );

    let pv = root_pv(&killed);
    assert!(!pv.is_empty());
    assert_legal_pv(&engine, &start, &pv);
}

#[test]
fn static_depth_inactive_kill_switch_matches_no_switch() {
    let engine = TestEngine::new();
    let start = engine.position(DEFAULT_FEN);
    let mut without_switch = Searcher::from(&start, MULTITHREADED);
    without_switch.search_config.search_mode = SearchMode::StaticDepth(5);
    without_switch.search_config.quiescence = false;
    without_switch.search_config.log_uci_diagnostics = false;

    let mut with_switch = Searcher::from(&start, MULTITHREADED);
    with_switch.search_config.search_mode = SearchMode::StaticDepth(5);
    with_switch.search_config.quiescence = false;
    with_switch.search_config.log_uci_diagnostics = false;
    let kill_switch = Arc::new(AtomicBool::new(false));

    without_switch.start_search(&engine.move_gen, &engine.zobrist, None);
    with_switch.start_search(
        &engine.move_gen,
        &engine.zobrist,
        Some(kill_switch.clone()),
    );

    let without_data = &without_switch.search_data[0];
    let with_data = &with_switch.search_data[0];
    assert!(!kill_switch.load(Relaxed));
    assert_eq!(root_pv(&with_switch), root_pv(&without_switch));
    assert_eq!(
        with_switch.collect_best_move(),
        without_switch.collect_best_move()
    );
    assert_eq!(
        with_switch.collect_ponder_move(),
        without_switch.collect_ponder_move()
    );
    assert_eq!(
        with_data.cumul_positions_searched,
        without_data.cumul_positions_searched
    );
    assert_eq!(with_data.pv, without_data.pv);
    assert_eq!(with_data.pv_ply_indices, without_data.pv_ply_indices);
    assert_eq!(with_data.history_table, without_data.history_table);
    assert_eq!(
        with_data.board_hash_history,
        without_data.board_hash_history
    );
    assert_eq!(with_data.positions_searched, 0);
    assert_eq!(without_data.positions_searched, 0);
}

#[test]
fn shallower_static_depth_reuses_deeper_completed_pv() {
    let engine = TestEngine::new();
    let start = engine.position(DEFAULT_FEN);
    let mut searcher = Searcher::from(&start, MULTITHREADED);
    searcher.search_config.quiescence = false;
    searcher.search_config.search_mode = SearchMode::StaticDepth(3);
    searcher.start_search(&engine.move_gen, &engine.zobrist, None);
    let deeper = root_pv(&searcher);

    searcher.search_config.search_mode = SearchMode::StaticDepth(2);
    searcher.start_search(&engine.move_gen, &engine.zobrist, None);

    assert_eq!(root_pv(&searcher), deeper);
}

#[test]
fn triangular_pv_boundaries_cover_max_depth() {
    assert_eq!(get_triang_pv_ply_idx_table(1), vec![0, 1]);
    assert_eq!(get_triang_pv_ply_idx_table(2), vec![0, 2, 3]);

    let max_depth_indices = get_triang_pv_ply_idx_table(MAX_SEARCH_DEPTH);
    assert_eq!(max_depth_indices.len(), MAX_SEARCH_DEPTH + 1);
    assert_eq!(max_depth_indices[MAX_SEARCH_DEPTH], TRIANG_PV_TABLE_SIZE);
    assert_eq!(
        max_depth_indices[MAX_SEARCH_DEPTH - 1],
        TRIANG_PV_TABLE_SIZE - 1
    );
}

#[test]
#[should_panic(expected = "exceeds MAX_SEARCH_DEPTH")]
fn static_depth_rejects_depth_above_table_capacity() {
    let engine = TestEngine::new();
    let _ = search_static_depth(&engine, DEFAULT_FEN, MAX_SEARCH_DEPTH + 1, false);
}

#[test]
fn static_depth_zero_preserves_existing_pv() {
    let engine = TestEngine::new();
    let start = engine.position(DEFAULT_FEN);
    let mut searcher = Searcher::from(&start, MULTITHREADED);
    searcher.search_config.quiescence = false;
    searcher.search_config.search_mode = SearchMode::StaticDepth(2);
    searcher.start_search(&engine.move_gen, &engine.zobrist, None);
    let completed = root_pv(&searcher);

    searcher.search_config.search_mode = SearchMode::StaticDepth(0);
    searcher.start_search(&engine.move_gen, &engine.zobrist, None);

    assert_eq!(root_pv(&searcher), completed);
}

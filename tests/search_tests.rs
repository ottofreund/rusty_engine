mod common;

use common::TestEngine;
use rusty_engine::{
    repr::_move::{self, NULL_MOVE},
    search::{search_config::SearchMode, searcher::Searcher},
    utils::fen_tool::DEFAULT_FEN,
};

fn search_static_depth(
    engine: &TestEngine,
    fen: &str,
    depth: usize,
    quiescence: bool,
) -> Searcher {
    let pos = engine.position(fen);
    let mut searcher = Searcher::from(&pos);
    searcher.search_config.search_mode = SearchMode::StaticDepth(depth);
    searcher.search_config.quiescence = quiescence;
    searcher.start_search(&engine.move_gen, &engine.zobrist, None);
    searcher
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
fn static_depth_quiescence_preserves_nominal_pv_length() {
    let engine = TestEngine::new();

    for depth in [1, 2] {
        let searcher = search_static_depth(&engine, DEFAULT_FEN, depth, true);
        let pv = &searcher.search_data[0].pv;
        let pv_len = pv
            .iter()
            .position(|mov| *mov == NULL_MOVE)
            .unwrap_or(pv.len());

        assert_eq!(pv_len, depth);
        assert_eq!(pv[depth], NULL_MOVE);
    }
}

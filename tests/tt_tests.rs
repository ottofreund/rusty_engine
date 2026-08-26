mod common;

use common::TestEngine;
use rusty_engine::{
    repr::{
        _move::{self, NULL_MOVE},
        position::Position,
    },
    search::{
        eval::{MATE_BOUND, MATE_EVAL},
        tt::{TTEntry, TTEntryType, TranspositionTable},
    },
    utils::fen_tool::DEFAULT_FEN,
};

fn legal_move(pos: &Position, uci: &str) -> u32 {
    pos.legal_search_moves()
        .iter()
        .copied()
        .find(|mov| _move::to_string(*mov, true) == uci)
        .unwrap_or_else(|| panic!("{uci} is not legal in the test position"))
}

fn store_exact_line(
    engine: &TestEngine,
    tt: &mut TranspositionTable,
    root: &Position,
    uci_moves: &[&str],
    root_depth: u8,
) -> Vec<u32> {
    let mut replay = root.clone();
    let mut moves = Vec::with_capacity(uci_moves.len());

    for (ply, uci) in uci_moves.iter().enumerate() {
        let mov = legal_move(&replay, uci);
        tt.store(TTEntry::new_packed(
            replay.zhash,
            mov,
            root_depth - ply as u8,
            TTEntryType::Exact,
            0,
            tt.generation,
        ));
        moves.push(mov);
        engine.make_search_move(&mut replay, mov);
    }

    moves
}

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
fn reconstruct_pv_replays_a_canonical_legal_line() {
    let engine = TestEngine::new();
    let root = engine.position(DEFAULT_FEN);
    let mut tt = TranspositionTable::default();
    let expected = store_exact_line(
        &engine,
        &mut tt,
        &root,
        &["e2e4", "e7e5", "g1f3"],
        3,
    );
    let mut pv = [NULL_MOVE; 3];

    let len = tt.reconstruct_pv(
        &root,
        &mut pv,
        &[root.zhash],
        &engine.move_gen,
        &engine.zobrist,
    );

    assert_eq!(len, expected.len());
    assert_eq!(pv.as_slice(), expected.as_slice());
}

#[test]
fn reconstruct_pv_replays_en_passant_and_later_castling() {
    let engine = TestEngine::new();

    let ep_root = engine.position("4k3/8/8/8/3p4/8/4P3/4K3 w - - 0 1");
    let mut ep_tt = TranspositionTable::default();
    let expected_ep = store_exact_line(
        &engine,
        &mut ep_tt,
        &ep_root,
        &["e2e4", "d4e3"],
        2,
    );
    assert!(_move::is_double_push(expected_ep[0]));
    assert!(_move::is_en_passant(expected_ep[1]));
    let mut ep_pv = [NULL_MOVE; 2];

    let ep_len = ep_tt.reconstruct_pv(
        &ep_root,
        &mut ep_pv,
        &[ep_root.zhash],
        &engine.move_gen,
        &engine.zobrist,
    );

    assert_eq!(ep_len, expected_ep.len());
    assert_eq!(ep_pv.as_slice(), expected_ep.as_slice());

    let castle_root = engine.position("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
    let mut castle_tt = TranspositionTable::default();
    let expected_castle = store_exact_line(
        &engine,
        &mut castle_tt,
        &castle_root,
        &["a1a2", "a8a7", "e1g1"],
        3,
    );
    assert!(_move::is_castle(expected_castle[2]));
    let mut castle_pv = [NULL_MOVE; 3];

    let castle_len = castle_tt.reconstruct_pv(
        &castle_root,
        &mut castle_pv,
        &[castle_root.zhash],
        &engine.move_gen,
        &engine.zobrist,
    );

    assert_eq!(castle_len, expected_castle.len());
    assert_eq!(castle_pv.as_slice(), expected_castle.as_slice());
}

#[test]
fn reconstruct_pv_stops_at_non_exact_or_too_shallow_entries() {
    let engine = TestEngine::new();
    let root = engine.position(DEFAULT_FEN);
    let mut child = root.clone();
    let root_move = legal_move(&root, "e2e4");
    engine.make_search_move(&mut child, root_move);
    let child_move = legal_move(&child, "e7e5");
    let mut tt = TranspositionTable::default();

    tt.store(TTEntry::new_packed(
        root.zhash,
        root_move,
        3,
        TTEntryType::Exact,
        0,
        tt.generation,
    ));
    tt.store(TTEntry::new_packed(
        child.zhash,
        child_move,
        2,
        TTEntryType::LowerBound,
        0,
        tt.generation,
    ));

    let mut pv = [NULL_MOVE; 3];
    let len = tt.reconstruct_pv(
        &root,
        &mut pv,
        &[root.zhash],
        &engine.move_gen,
        &engine.zobrist,
    );

    assert_eq!(len, 1);
    assert_eq!(pv, [root_move, NULL_MOVE, NULL_MOVE]);

    let mut shallow_tt = TranspositionTable::default();
    shallow_tt.store(TTEntry::new_packed(
        root.zhash,
        root_move,
        2,
        TTEntryType::Exact,
        0,
        shallow_tt.generation,
    ));
    let mut stale_tail = [NULL_MOVE, child_move, child_move];
    let shallow_len = shallow_tt.reconstruct_pv(
        &root,
        &mut stale_tail,
        &[root.zhash],
        &engine.move_gen,
        &engine.zobrist,
    );

    assert_eq!(shallow_len, 0);
    assert_eq!(stale_tail, [NULL_MOVE; 3]);

    let mut illegal_tt = TranspositionTable::default();
    illegal_tt.store(TTEntry::new_packed(
        root.zhash,
        child_move,
        1,
        TTEntryType::Exact,
        0,
        illegal_tt.generation,
    ));
    let mut illegal_pv = [NULL_MOVE; 1];
    let illegal_len = illegal_tt.reconstruct_pv(
        &root,
        &mut illegal_pv,
        &[root.zhash],
        &engine.move_gen,
        &engine.zobrist,
    );

    assert_eq!(illegal_len, 0);
    assert_eq!(illegal_pv, [NULL_MOVE]);
}

#[test]
fn reconstruct_pv_stops_after_fifty_move_draw() {
    let engine = TestEngine::new();
    let root = engine.position("4k3/8/8/8/8/8/4N3/4K3 w - - 99 1");
    let mut tt = TranspositionTable::default();
    let expected = store_exact_line(
        &engine,
        &mut tt,
        &root,
        &["e2c3", "e8d7"],
        2,
    );
    let mut tt_only_pv = [NULL_MOVE; 2];

    let tt_only_len = tt.reconstruct_pv(
        &root,
        &mut tt_only_pv,
        &[root.zhash],
        &engine.move_gen,
        &engine.zobrist,
    );

    assert_eq!(tt_only_len, 0);
    assert_eq!(tt_only_pv, [NULL_MOVE; 2]);

    let mut pv = [expected[0], NULL_MOVE];

    let len = tt.reconstruct_pv(
        &root,
        &mut pv,
        &[root.zhash],
        &engine.move_gen,
        &engine.zobrist,
    );

    assert_eq!(len, 1);
    assert_eq!(pv, [expected[0], NULL_MOVE]);
}

#[test]
fn reconstruct_pv_stops_after_threefold_repetition() {
    let engine = TestEngine::new();
    let root = engine.position("4k1n1/8/8/8/8/8/8/4K1N1 w - - 0 1");
    let mut tt = TranspositionTable::default();
    let cycle = ["g1f3", "g8f6", "f3g1", "f6g8"];
    let expected_cycle = store_exact_line(&engine, &mut tt, &root, &cycle, 10);
    let mut pv = [NULL_MOVE; 10];

    let len = tt.reconstruct_pv(
        &root,
        &mut pv,
        &[root.zhash],
        &engine.move_gen,
        &engine.zobrist,
    );

    assert_eq!(len, 8);
    assert_eq!(&pv[..4], expected_cycle.as_slice());
    assert_eq!(&pv[4..8], expected_cycle.as_slice());
    assert_eq!(&pv[8..], &[NULL_MOVE; 2]);
}

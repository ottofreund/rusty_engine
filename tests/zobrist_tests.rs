mod common;

use common::TestEngine;
use rusty_engine::{
    repr::{_move, position::Position, types::W_QUEEN},
    utils::fen_tool::{self, DEFAULT_FEN},
};

const DEFAULT_BLACK_TO_MOVE_FEN: &str =
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1";
const CASTLING_FEN: &str = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
const NO_CASTLING_FEN: &str = "r3k2r/8/8/8/8/8/8/R3K2R w - - 0 1";
const EN_PASSANT_FEN: &str = "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1";
const NO_EN_PASSANT_FEN: &str = "4k3/8/8/3pP3/8/8/8/4K3 w - - 0 1";
const CAPTURE_FEN: &str = "4k3/8/8/3p4/4P3/8/8/4K3 w - - 0 1";
const PROMOTION_FEN: &str = "4k3/P7/8/8/8/8/8/4K3 w - - 0 1";

#[test]
fn initialized_hash_matches_full_recompute() {
    let engine = TestEngine::new();

    for fen in [
        DEFAULT_FEN,
        DEFAULT_BLACK_TO_MOVE_FEN,
        CASTLING_FEN,
        EN_PASSANT_FEN,
        CAPTURE_FEN,
        PROMOTION_FEN,
    ] {
        let pos = engine.position(fen);
        assert_hash_matches_recompute(&engine, &pos);
    }
}

#[test]
fn side_to_move_castling_and_en_passant_are_part_of_hash() {
    let engine = TestEngine::new();

    assert_ne!(
        engine.position(DEFAULT_FEN).board.zhash,
        engine.position(DEFAULT_BLACK_TO_MOVE_FEN).board.zhash,
        "side to move should affect hash"
    );
    assert_ne!(
        engine.position(CASTLING_FEN).board.zhash,
        engine.position(NO_CASTLING_FEN).board.zhash,
        "castling rights should affect hash"
    );
    assert_ne!(
        engine.position(EN_PASSANT_FEN).board.zhash,
        engine.position(NO_EN_PASSANT_FEN).board.zhash,
        "en passant file should affect hash"
    );
}

#[test]
fn incremental_hash_matches_recompute_through_move_sequence() {
    let engine = TestEngine::new();
    let mut pos = engine.position(DEFAULT_FEN);
    let mut history = Vec::new();

    for (from, to) in [
        (square('e', 2), square('e', 4)),
        (square('c', 7), square('c', 5)),
        (square('g', 1), square('f', 3)),
        (square('d', 7), square('d', 6)),
    ] {
        let mov = legal_move_matching(&pos, |mov| {
            _move::get_init(mov) == from && _move::get_target(mov) == to
        });
        history.push((mov, pos.board.zhash, fen_tool::board_to_fen(&pos.board)));

        engine.make_search_move(&mut pos, mov);
        assert_hash_matches_recompute(&engine, &pos);
    }

    while let Some((mov, expected_hash, expected_fen)) = history.pop() {
        engine.unmake_move(&mut pos, mov);
        assert_hash_matches_recompute(&engine, &pos);
        assert_eq!(pos.board.zhash, expected_hash);
        assert_eq!(fen_tool::board_to_fen(&pos.board), expected_fen);
    }
}

#[test]
fn castling_hash_updates_and_restores() {
    assert_make_unmake_hash_round_trip(CASTLING_FEN, |pos| {
        legal_move_matching(pos, |mov| {
            _move::is_castle(mov) && _move::is_short_castle(mov)
        })
    });
}

#[test]
fn capture_hash_updates_and_restores() {
    assert_make_unmake_hash_round_trip(CAPTURE_FEN, |pos| {
        legal_move_matching(pos, |mov| {
            _move::get_init(mov) == square('e', 4)
                && _move::get_target(mov) == square('d', 5)
                && _move::is_eating(mov)
        })
    });
}

#[test]
fn en_passant_hash_updates_and_restores() {
    assert_make_unmake_hash_round_trip(EN_PASSANT_FEN, |pos| {
        legal_move_matching(pos, |mov| {
            _move::get_init(mov) == square('e', 5)
                && _move::get_target(mov) == square('d', 6)
                && _move::is_en_passant(mov)
        })
    });
}

#[test]
fn promotion_hash_updates_and_restores() {
    assert_make_unmake_hash_round_trip(PROMOTION_FEN, |pos| {
        legal_move_matching(pos, |mov| {
            _move::get_init(mov) == square('a', 7)
                && _move::get_target(mov) == square('a', 8)
                && _move::is_promotion(mov)
                && _move::get_promoted_piece(mov) == W_QUEEN
        })
    });
}

fn assert_make_unmake_hash_round_trip<F>(fen: &str, select_move: F)
where
    F: FnOnce(&Position) -> u32,
{
    let engine = TestEngine::new();
    let mut pos = engine.position(fen);
    let before_hash = pos.board.zhash;
    let before_fen = fen_tool::board_to_fen(&pos.board);
    let mov = select_move(&pos);

    engine.make_search_move(&mut pos, mov);
    assert_hash_matches_recompute(&engine, &pos);
    assert_ne!(
        pos.board.zhash,
        before_hash,
        "hash should change after {}",
        _move::to_string(mov)
    );

    engine.unmake_move(&mut pos, mov);
    assert_hash_matches_recompute(&engine, &pos);
    assert_eq!(pos.board.zhash, before_hash);
    assert_eq!(fen_tool::board_to_fen(&pos.board), before_fen);
}

fn assert_hash_matches_recompute(engine: &TestEngine, pos: &Position) {
    assert_eq!(pos.board.zhash, engine.recomputed_hash(&pos.board));
}

fn legal_move_matching<F>(pos: &Position, matches: F) -> u32
where
    F: Fn(u32) -> bool,
{
    pos.legal_search_moves()
        .iter()
        .copied()
        .find(|mov| matches(*mov))
        .unwrap_or_else(|| {
            let legal_moves = pos
                .legal_search_moves()
                .iter()
                .map(|mov| _move::to_string(*mov))
                .collect::<Vec<_>>()
                .join(", ");
            panic!("no matching legal move found. Legal moves: {legal_moves}");
        })
}

fn square(file: char, rank: u32) -> u32 {
    file as u32 - 'a' as u32 + 8 * (rank - 1)
}

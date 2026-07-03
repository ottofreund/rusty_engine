mod common;

use common::TestEngine;
use rusty_engine::{
    repr::_move,
    utils::fen_tool::{self, DEFAULT_FEN},
};

#[test]
fn decoding_works() {
    let engine = TestEngine::new();
    let starting_pos_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let board = engine.board(starting_pos_fen);
    let correct_default = engine.default_board();
    assert_eq!(board.pieces, correct_default.pieces);
    assert_eq!(board.half_move_clock, 0);

    let en_passant_fen = "rnbqkbnr/pppppppp/8/8/4Pp2/8/PPPP2PP/RNBQKBNR w KQkq f6";
    let en_passant_board = engine.board(en_passant_fen);
    assert_eq!(en_passant_board.ep_square, Some(45));
    assert_eq!(en_passant_board.half_move_clock, 0);

    let half_move_clock_fen = "4k3/8/8/8/8/8/8/4K3 w - - 17 42";
    let half_move_clock_board = engine.board(half_move_clock_fen);
    assert_eq!(half_move_clock_board.half_move_clock, 17);
}

#[test]
fn encoding_works() {
    let engine = TestEngine::new();
    let board = engine.default_board();

    assert_eq!(
        fen_tool::board_to_fen(&board),
        DEFAULT_FEN
    );

    let en_passant_fen = "rnbqkbnr/pppppppp/8/8/4Pp2/8/PPPP2PP/RNBQKBNR w KQkq f6 0 1";
    let en_passant_board = engine.board(en_passant_fen);

    assert_eq!(fen_tool::board_to_fen(&en_passant_board), en_passant_fen);

    let half_move_clock_fen = "4k3/8/8/8/8/8/8/4K3 w - - 17 42";
    let half_move_clock_board = engine.board(half_move_clock_fen);
    assert_eq!(
        fen_tool::board_to_fen(&half_move_clock_board),
        "4k3/8/8/8/8/8/8/4K3 w - - 17 1"
    );
}

#[test]
fn half_move_clock_updates_and_unmakes() {
    let engine = TestEngine::new();
    let mut pos = engine.position("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 12 1");

    let knight_move = legal_move_matching(&pos, square('g', 1), square('f', 3));
    engine.make_search_move(&mut pos, knight_move);
    assert_eq!(pos.board.half_move_clock, 13);

    engine.unmake_move(&mut pos, knight_move);
    assert_eq!(pos.board.half_move_clock, 12);

    let pawn_move = legal_move_matching(&pos, square('e', 2), square('e', 4));
    engine.make_search_move(&mut pos, pawn_move);
    assert_eq!(pos.board.half_move_clock, 0);
}

fn legal_move_matching(pos: &rusty_engine::repr::position::Position, from: u32, to: u32) -> u32 {
    pos.legal_search_moves()
        .iter()
        .copied()
        .find(|mov| _move::get_init(*mov) == from && _move::get_target(*mov) == to)
        .expect("expected legal move")
}

fn square(file: char, rank: u32) -> u32 {
    file as u32 - 'a' as u32 + 8 * (rank - 1)
}

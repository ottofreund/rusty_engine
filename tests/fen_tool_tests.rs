mod common;

use common::TestEngine;
use rusty_engine::utils::fen_tool;

#[test]
fn decoding_works() {
    let engine = TestEngine::new();
    let starting_pos_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let board = engine.board(starting_pos_fen);
    let correct_default = engine.default_board();
    assert_eq!(board.pieces, correct_default.pieces);

    let en_passant_fen = "rnbqkbnr/pppppppp/8/8/4Pp2/8/PPPP2PP/RNBQKBNR w KQkq f6";
    let en_passant_board = engine.board(en_passant_fen);
    assert_eq!(en_passant_board.ep_square, Some(45));
}

#[test]
fn encoding_works() {
    let engine = TestEngine::new();
    let board = engine.default_board();

    assert_eq!(
        fen_tool::board_to_fen(&board),
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -"
    );

    let en_passant_fen = "rnbqkbnr/pppppppp/8/8/4Pp2/8/PPPP2PP/RNBQKBNR w KQkq f6";
    let en_passant_board = engine.board(en_passant_fen);

    assert_eq!(fen_tool::board_to_fen(&en_passant_board), en_passant_fen);
}

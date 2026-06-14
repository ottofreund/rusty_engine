use rusty_engine::repr::board::Board;
use rusty_engine::repr::move_gen::MoveGen;
use rusty_engine::repr::*;
use rusty_engine::utils::fen_tool;

#[test]
fn decoding_works() {
    let move_gen: MoveGen = move_gen::MoveGen::init();
    let starting_pos_fen: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let board: Board = fen_tool::fen_to_board(starting_pos_fen.to_string(), &move_gen).expect("");
    let correct_default: Board = Board::default_board(&move_gen);
    assert_eq!(board.pieces, correct_default.pieces);

    
    let en_passant_fen = "rnbqkbnr/pppppppp/8/8/4Pp2/8/PPPP2PP/RNBQKBNR w KQkq f6";
    let en_passant_board =
        fen_tool::fen_to_board(en_passant_fen.to_string(), &move_gen).expect("valid FEN");
    assert_eq!(en_passant_board.ep_square, Some(45));
}

#[test]
fn encoding_works() {
    let move_gen = MoveGen::init();
    let board = Board::default_board(&move_gen);

    assert_eq!(
        fen_tool::board_to_fen(&board),
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -"
    );

    let en_passant_fen = "rnbqkbnr/pppppppp/8/8/4Pp2/8/PPPP2PP/RNBQKBNR w KQkq f6";
    let en_passant_board =
        fen_tool::fen_to_board(en_passant_fen.to_string(), &move_gen).expect("valid FEN");

    assert_eq!(fen_tool::board_to_fen(&en_passant_board), en_passant_fen);
}

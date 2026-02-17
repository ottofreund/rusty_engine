use rusty_engine::repr::board::Board;
use rusty_engine::repr::move_gen::MoveGen;
use rusty_engine::repr::*;
use rusty_engine::utils::fen_tool;
use crate::board::Board;

#[test]
fn decoding_works() {
    let move_gen: MoveGen = move_gen::MoveGen::init();
    let starting_pos_fen: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let board: Board = board::fen_to_board(starting_pos_fen.to_string(), &move_gen).expect("");
    let correct_default: Board = Board::default_board(&move_gen);
    assert_eq!(board.pieces, correct_default.pieces);
}

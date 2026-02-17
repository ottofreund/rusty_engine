use rusty_engine::repr::board::Board;
use rusty_engine::repr::game::Game;
use rusty_engine::repr::move_gen::MoveGen;
use rusty_engine::repr::*;
use rusty_engine::utils::fen_tool::{DEFAULT_FEN};
use crate::types::Color;

#[test]
fn default_pos_perft_correct() {
    let game: Game = Game::game_with(DEFAULT_FEN).unwrap();
    
}

fn go_perft(depth: u32, game: &mut Game, expected: u32) -> bool {
    let mut found: u32 = 0;
    fn inner(d: u32) {
        
    }
    return found == expected;
}
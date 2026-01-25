mod repr;
use repr::*;
use repr::board::Board;
use rusty_engine::{repr::{board::square_to_string, game::Game}, ui::{self, app::AppState}};
use repr::magic_bb_loader::MagicBitboard;

use rusty_engine::ui::*;


use rand::prelude::*;

fn main() {
    app::run_fr();
    //let game: Game = Game::default();
/* 
    game.legal_moves.iter().for_each(|mov| println!("{}", _move::to_string(*mov)));
    game.try_make_move(6, 21);
    game.legal_moves.iter().for_each(|mov| println!("{}", _move::to_string(*mov)));
    game.try_make_move(57, 42);
    game.legal_moves.iter().for_each(|mov | println!("{}", _move::to_string(*mov)));*/

}

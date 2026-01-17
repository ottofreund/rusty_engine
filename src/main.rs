mod repr;
use repr::*;
use repr::board::Board;
use rusty_engine::repr::{board::{HIGHER_FILES, HIGHER_RANKS, LOWER_FILES, LOWER_RANKS, square_to_string}, game::Game};
use repr::magic_bb_loader::MagicBitboard;

use crate::repr::board::EDGES;

use rand::prelude::*;

fn main() {
    let game: Game = Game::init_default();


    /* let mut sqr: u32 = 20;
    println!("Precomputed rook map holds {} entries", gen.rook_slide_bbs.len());
    println!("With edges: \n{} \n No edges:\n{}", bitboard::bb_to_string(gen.attack_bbs[3][sqr as usize]), bitboard::bb_to_string(gen.rook_bbs_no_edges[sqr as usize])); */


    /* let board: Board = board::Board::default_board();
    let mut res_vec: Vec<u32> = vec!();
    move_gen::pseudolegal_pawn(8, types::Color::White, &board, &gen, &mut res_vec);
    let str_mapped: Vec<String> = res_vec.iter().map(|mov: &;u32| _move::to_string(*mov)).collect();
    println!("Got moves: {:?}", str_mapped); */
}

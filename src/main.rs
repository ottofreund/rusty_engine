mod repr;
use repr::*;
use repr::board::Board;
use rusty_engine::repr::board::{HIGHER_FILES, HIGHER_RANKS, LOWER_FILES, LOWER_RANKS, square_to_string};
use repr::magic_bb_loader::MagicBitboard;

use crate::repr::board::EDGES;

use rand::prelude::*;

fn main() {
    let mut gen: move_gen::MoveGen = repr::move_gen::MoveGen::init();
    let mut board: Board = Board::default_board();
    let moves: Vec<u32> = gen.get_all_pseudolegal(&mut board, types::Color::Black);
    println!("Black moves:\n");
    for mov in moves.iter() {
        println!("{}\n", _move::to_string(*mov));
    }
    //LOWER_FILES.iter().for_each(|higher_bb: &u64| println!("{}", bitboard::bb_to_string(*higher_bb)));
    //println!("rook_slide_bbs[0][0]:\n{}", bitboard::bb_to_string(72340172838076926));

    /* let mut sqr: u32 = 20;
    println!("Precomputed rook map holds {} entries", gen.rook_slide_bbs.len());
    println!("With edges: \n{} \n No edges:\n{}", bitboard::bb_to_string(gen.attack_bbs[3][sqr as usize]), bitboard::bb_to_string(gen.rook_bbs_no_edges[sqr as usize])); */


    /* let board: Board = board::Board::default_board();
    let mut res_vec: Vec<u32> = vec!();
    move_gen::pseudolegal_pawn(8, types::Color::White, &board, &gen, &mut res_vec);
    let str_mapped: Vec<String> = res_vec.iter().map(|mov: &;u32| _move::to_string(*mov)).collect();
    println!("Got moves: {:?}", str_mapped); */
}

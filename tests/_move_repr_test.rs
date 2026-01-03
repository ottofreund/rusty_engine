use rusty_engine::repr::*;
use crate::types::Color;

#[test]
fn encoding_works() {
    //non eating
    assert_eq!(_move::create(57, 42, None, Color::White, 0), 2147486393);
    assert_eq!(_move::create(0, 63, None, Color::White, 0), 2147487680);
    //eating
    assert_eq!(_move::create(57, 42, Some(4), Color::Black, 6), 402692793); //eat white queen
    assert_eq!(_move::create(0, 63, Some(9), Color::White, 0), 2147565504); //eat black rook
    //promoting
    assert_eq!(_move::create_promotion(49, 57, None, 10, Color::Black, 6), 413666929); //to black queen
    assert_eq!(_move::create_promotion(49, 58, Some(9), 4, Color::White, 0), 2152283825); //to white queen, eating a black rook
}

#[test]
fn decoding_works() {
    //non-eating promotion move
    let m1 = _move::create_promotion(49, 57, None, 10, Color::Black, 6);
    assert_eq!(_move::get_init(m1), 49);
    assert_eq!(_move::get_target(m1), 57);
    assert_eq!(_move::is_eating(m1), false);
    assert_eq!(_move::eaten_piece(m1), None);
    assert_eq!(_move::is_promotion(m1), true);
    assert_eq!(_move::is_white_move(m1), false);
    //eating move
    let m2 = _move::create(57, 42, Some(4), Color::Black, 6);
    assert_eq!(_move::get_init(m2), 57);
    assert_eq!(_move::get_target(m2), 42);
    assert_eq!(_move::is_eating(m2), true);
    assert_eq!(_move::eaten_piece(m2), Some(4));
    assert_eq!(_move::is_promotion(m2), false);
    assert_eq!(_move::is_white_move(m2), false);
    //castling moves
    let m3 = _move::create_castling(Color::Black, true); //white short
    assert_eq!(_move::is_castle(m3), true);
    assert_eq!(_move::is_short_castle(m3), true);
    assert_eq!(_move::is_long_castle(m3), false)
}
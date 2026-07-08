use rusty_engine::repr::{
    types::{BLACK, WHITE, W_PAWN, W_QUEEN},
    *,
};

#[test]
fn encoding_works() {
    //non eating
    assert_eq!(_move::create(57, 42, false, WHITE, 0), 2147486393);
    assert_eq!(_move::create(0, 63, false, WHITE, 0), 2147487680);
    //eating
    assert_eq!(_move::create(57, 42, true, BLACK, 6), 402660025); //eat white queen
    assert_eq!(_move::create(0, 63, true, WHITE, 0), 2147491776); //eat black rook
                                                                  //promoting
    assert_eq!(
        _move::create_promotion(49, 57, false, 10, BLACK, 6),
        446697073
    ); //to black queen
    assert_eq!(
        _move::with_eaten_piece(_move::create_promotion(49, 58, true, 4, WHITE, 0), 9),
        2166513329
    ); //to white queen, eating a black rook
}

#[test]
fn decoding_works() {
    //non-eating promotion move
    let m1 = _move::create_promotion(49, 57, false, 10, BLACK, 6);
    assert_eq!(_move::get_init(m1), 49);
    assert_eq!(_move::get_target(m1), 57);
    assert_eq!(_move::is_eating(m1), false);
    assert_eq!(_move::is_promotion(m1), true);
    assert_eq!(_move::is_white_move(m1), false);
    //eating move
    let m2 = _move::create(57, 42, true, BLACK, 6);
    assert_eq!(_move::get_init(m2), 57);
    assert_eq!(_move::get_target(m2), 42);
    assert_eq!(_move::is_eating(m2), true);
    assert_eq!(_move::is_promotion(m2), false);
    assert_eq!(_move::is_white_move(m2), false);
    //castling moves
    let m3 = _move::create_castling(BLACK, true); //white short
    assert_eq!(_move::is_castle(m3), true);
    assert_eq!(_move::is_short_castle(m3), true);
    assert_eq!(_move::is_long_castle(m3), false)
}

#[test]
fn to_string_supports_readable_and_uci_formats() {
    let quiet = _move::create(12, 28, false, WHITE, W_PAWN);
    assert_eq!(_move::to_string(quiet, false), "P(e2) -> e4");
    assert_eq!(_move::to_string(quiet, true), "e2e4");

    let capture = _move::create(12, 28, true, WHITE, W_PAWN);
    assert_eq!(_move::to_string(capture, false), "P(e2) x e4");
    assert_eq!(_move::to_string(capture, true), "e2e4");

    let promotion = _move::create_promotion(52, 60, false, W_QUEEN, WHITE, W_PAWN);
    assert_eq!(
        _move::to_string(promotion, false),
        "P(e7) -> e8 -- promoting to Q"
    );
    assert_eq!(_move::to_string(promotion, true), "e7e8q");

    assert_eq!(
        _move::to_string(_move::create_castling(WHITE, true), true),
        "e1g1"
    );
    assert_eq!(_move::to_string(_move::NULL_MOVE, false), "NULL_MOVE");
    assert_eq!(_move::to_string(_move::NULL_MOVE, true), "0000");
}

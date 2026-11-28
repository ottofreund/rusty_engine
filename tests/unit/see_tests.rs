use super::*;
use crate::utils::fen_tool;

#[test]
fn promoting_recaptures_use_promoted_value_and_gain() {
    let move_gen = MoveGen::init();
    let cases = [
        (
            "3Q3k/4P3/8/8/8/8/K7/3r4 b - - 0 1",
            _move::with_eaten_piece(
                _move::create(3, 59, true, BLACK, B_ROOK_U as u32),
                W_QUEEN_U as u32,
            ),
        ),
        (
            "3R4/k7/8/8/8/8/4p3/3q3K w - - 0 1",
            _move::with_eaten_piece(
                _move::create(59, 3, true, WHITE, W_ROOK_U as u32),
                B_QUEEN_U as u32,
            ),
        ),
    ];

    for (fen, initiating_move) in cases {
        let board = fen_tool::fen_to_board(fen.to_owned(), &move_gen)
            .expect("valid promotion-recapture position");

        assert!(!SeeWorker.see_positive(initiating_move, &board, &move_gen));
    }
}

#[test]
fn black_rook_reveals_a_second_attacker_after_recapturing() {
    let move_gen = MoveGen::init();
    let mov = _move::with_eaten_piece(_move::create(3, 35, true, WHITE, W_ROOK), B_PAWN);
    for (fen, expected) in [
        ("6k1/8/3r4/3p4/5N2/8/8/3R2K1 w - - 0 1", true),
        ("6k1/3r4/3r4/3p4/5N2/8/8/3R2K1 w - - 0 1", false),
    ] {
        let board = fen_tool::fen_to_board(fen.to_owned(), &move_gen).unwrap();
        assert_eq!(SeeWorker.see_positive(mov, &board, &move_gen), expected);
    }
}

#[test]
fn pinned_defenders_cannot_recapture_off_the_pin_ray() {
    let move_gen = MoveGen::init();
    let mov = _move::with_eaten_piece(_move::create(3, 35, true, WHITE, W_QUEEN), B_PAWN);
    for (fen, expected) in [
        ("5k2/8/5n2/3p4/8/8/8/3Q2K1 w - - 0 1", false),
        ("5k2/8/5n2/3p4/8/8/8/3Q1RK1 w - - 0 1", true),
    ] {
        let board = fen_tool::fen_to_board(fen.to_owned(), &move_gen).unwrap();
        assert_eq!(SeeWorker.see_positive(mov, &board, &move_gen), expected);
    }
}

#[test]
fn king_cannot_recapture_on_a_defended_square() {
    let move_gen = MoveGen::init();
    let mov = _move::with_eaten_piece(_move::create(24, 28, true, WHITE, W_QUEEN), B_PAWN);
    for (fen, expected) in [
        ("8/8/8/5k2/Q3p3/8/8/6K1 w - - 0 1", false),
        ("8/8/8/5k2/Q3p3/8/8/4R1K1 w - - 0 1", true),
    ] {
        let board = fen_tool::fen_to_board(fen.to_owned(), &move_gen).unwrap();
        assert_eq!(SeeWorker.see_positive(mov, &board, &move_gen), expected);
    }
}

#[test]
fn en_passant_removes_the_victim_before_rook_xrays() {
    let move_gen = MoveGen::init();
    let mov = _move::with_eaten_piece(_move::create_en_passant(36, 43, WHITE, W_PAWN), B_PAWN);
    let board = fen_tool::fen_to_board("6k1/8/8/3pP3/8/8/8/3r2K1 w - d6 0 1".to_owned(), &move_gen)
        .unwrap();
    assert!(SeeWorker.see_ge(mov, 0, &board, &move_gen));
    assert!(!SeeWorker.see_ge(mov, 1, &board, &move_gen));
}

#[test]
fn quiet_promotion_accounts_for_an_immediate_capture() {
    let move_gen = MoveGen::init();
    let mov = _move::create_promotion(48, 56, false, W_QUEEN, WHITE, W_PAWN);
    let board =
        fen_tool::fen_to_board("1r4k1/P7/8/8/8/8/8/6K1 w - - 0 1".to_owned(), &move_gen).unwrap();
    assert!(!SeeWorker.see_positive(mov, &board, &move_gen));
    assert!(SeeWorker.see_ge(mov, -100, &board, &move_gen));
}

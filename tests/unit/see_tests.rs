use super::*;
use crate::utils::{fen_tool, zobrist::Zobrist};

#[test]
fn promoting_recaptures_use_promoted_value_and_gain() {
    let move_gen = MoveGen::init();
    let zobrist = Zobrist::default();
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
        let board = fen_tool::fen_to_board(fen.to_owned(), &move_gen, &zobrist)
            .expect("valid promotion-recapture position");

        assert!(!SeeWorker::default().see_positive(initiating_move, &board, &move_gen,));
    }
}

#[test]
fn black_discovered_attackers_preserve_value_groups_and_lvp() {
    let mut worker = SeeWorker::default();
    worker.piece_s_indices_black = [NO_ENTRIES_IDX; NOF_PIECE_TYPES_U];

    worker.add_discovered_attacker(16, B_QUEEN_U);
    worker.add_discovered_attacker(8, B_PAWN_U);
    worker.add_discovered_attacker(24, B_ROOK_U);
    worker.add_discovered_attacker(32, B_PAWN_U);

    assert_eq!(worker.attackers_black, vec![16, 24, 8, 32]);
    assert_eq!(worker.lvp_black, Some(B_PAWN_U));
    assert_eq!(worker.piece_s_indices_black[W_QUEEN_U], 0);
    assert_eq!(worker.piece_s_indices_black[W_ROOK_U], 1);
    assert_eq!(worker.piece_s_indices_black[W_PAWN_U], 2);
    assert_eq!(worker.piece_s_indices_black[W_KNIGHT_U], NO_ENTRIES_IDX);
    assert_eq!(worker.piece_s_indices_black[W_BISHOP_U], NO_ENTRIES_IDX);

    assert_eq!(worker.pop_lvp(BLACK), (32, W_PAWN_U));
    assert_eq!(worker.lvp_black, Some(B_PAWN_U));
    assert_eq!(worker.pop_lvp(BLACK), (8, W_PAWN_U));
    assert_eq!(worker.lvp_black, Some(B_ROOK_U));
    assert_eq!(worker.pop_lvp(BLACK), (24, W_ROOK_U));
    assert_eq!(worker.lvp_black, Some(B_QUEEN_U));
    assert_eq!(worker.pop_lvp(BLACK), (16, W_QUEEN_U));
    assert_eq!(worker.lvp_black, None);
    assert_eq!(worker.total_attackers, 0);
}

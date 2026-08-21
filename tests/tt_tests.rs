use rusty_engine::search::{
    eval::{MATE_BOUND, MATE_EVAL},
    tt::TranspositionTable,
};

#[test]
fn winning_mate_score_rebases_between_plies() {
    // Mate in 3, first encountered at ply 5, is mate at root ply 8.
    let score_at_old_ply = MATE_EVAL - 8;
    let stored = TranspositionTable::score_to_tt(score_at_old_ply, 5);

    assert_eq!(stored, MATE_EVAL - 3);
    assert_eq!(TranspositionTable::score_from_tt(stored, 2), MATE_EVAL - 5);
}

#[test]
fn losing_mate_score_rebases_between_plies() {
    // Mated in 4, first encountered at ply 5, is mate at root ply 9.
    let score_at_old_ply = -MATE_EVAL + 9;
    let stored = TranspositionTable::score_to_tt(score_at_old_ply, 5);

    assert_eq!(stored, -MATE_EVAL + 4);
    assert_eq!(TranspositionTable::score_from_tt(stored, 2), -MATE_EVAL + 6);
}

#[test]
fn non_mate_scores_are_not_rebased() {
    for score in [-MATE_BOUND + 1, -731, 0, 946, MATE_BOUND - 1] {
        let stored = TranspositionTable::score_to_tt(score, 17);

        assert_eq!(stored, score);
        assert_eq!(TranspositionTable::score_from_tt(stored, 4), score);
    }
}

#[test]
fn mate_bound_is_inclusive() {
    assert_eq!(
        TranspositionTable::score_to_tt(MATE_BOUND, 7),
        MATE_BOUND + 7,
    );
    assert_eq!(
        TranspositionTable::score_to_tt(-MATE_BOUND, 7),
        -MATE_BOUND - 7,
    );
}

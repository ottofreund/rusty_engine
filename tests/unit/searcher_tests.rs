use super::*;

fn search(
    searcher: &mut Searcher,
    depth: i16,
    move_end: Option<usize>,
    beta: i16,
    move_gen: &MoveGen,
    zobrist: &Zobrist,
) -> i16 {
    Searcher::negamax(
        depth,
        0,
        1,
        ALPHA_INIT,
        beta,
        true,
        move_end,
        false,
        false,
        true,
        &[],
        &mut searcher.positions[0],
        &searcher.evaluator,
        &mut searcher.search_data[0],
        move_gen,
        zobrist,
        &SearchControl::new(None, None),
        &mut searcher.tt,
    )
}

#[test]
fn reduced_quiescence_leaves_quiet_tt_move_for_full_depth() {
    let move_gen = MoveGen::init();
    let zobrist = Zobrist::default();
    let pos = Position::from("7k/1P6/8/8/4p3/3P4/8/K7 w - - 0 1", &move_gen, &zobrist).unwrap();
    let bounds = pos.search_move_bounds();
    let tactical_moves: Vec<u32> = pos
        .legal_search_moves()
        .iter()
        .copied()
        .filter(|&mov| _move::is_eating(mov) || _move::is_promotion(mov))
        .collect();
    assert!(tactical_moves.iter().any(|&mov| _move::is_eating(mov)));
    assert!(tactical_moves.iter().any(|&mov| _move::is_promotion(mov)));
    let quiet_move = *pos
        .legal_search_moves()
        .iter()
        .find(|&&mov| !_move::is_eating(mov) && !_move::is_promotion(mov))
        .unwrap();
    let mut searcher = Searcher::from(&pos, false);
    let tt_entry = TTEntry::new_packed(
        Zobrist::zkey50_adjusted(pos.zhash, pos.board.half_move_clock),
        quiet_move,
        1,
        TTEntryType::Exact,
        12_345,
        searcher.tt.generation,
    );
    searcher.tt.store(tt_entry);

    let move_end = Searcher::reduced_move_end(&mut searcher.positions[0], 0, true);
    let tactical_end = move_end.unwrap();
    assert_eq!(
        searcher.positions[0].move_arr[bounds.0..tactical_end],
        tactical_moves
    );
    assert_ne!(
        search(&mut searcher, 0, move_end, BETA_INIT, &move_gen, &zobrist),
        tt_entry.score,
    );
    assert!(searcher.search_data[0].sel_depth > 0);
    assert_eq!(searcher.positions[0].search_move_bounds(), bounds);
    assert!(searcher.positions[0].move_arr[tactical_end..bounds.1].contains(&quiet_move));

    searcher.tt.store(tt_entry);
    assert_eq!(
        search(&mut searcher, 1, None, BETA_INIT, &move_gen, &zobrist),
        tt_entry.score,
    );
    assert_eq!(searcher.search_data[0].pv[0], quiet_move);
}

#[test]
fn empty_tactical_prefix_reaches_stand_pat_and_draw_checks() {
    let move_gen = MoveGen::init();
    let zobrist = Zobrist::default();
    let pos = Position::from("7k/8/8/8/8/8/P7/K7 w - - 99 1", &move_gen, &zobrist).unwrap();
    let eval =
        Evaluator::default().eval(pos.board.pieces, pos.board.turn, pos.board.late_game_phase);
    assert_ne!(eval, 0);
    let (start, end) = pos.search_move_bounds();
    assert!(start < end);
    let mut searcher = Searcher::from(&pos, false);
    let move_end = Searcher::reduced_move_end(&mut searcher.positions[0], 0, true);
    assert_eq!(move_end, Some(start));

    assert_eq!(
        search(&mut searcher, 0, move_end, eval, &move_gen, &zobrist),
        eval
    );
    assert_eq!(searcher.search_data[0].stand_pat_cutoffs, 1);

    searcher.positions[0].board.half_move_clock = 100;
    assert_eq!(
        search(&mut searcher, 0, move_end, BETA_INIT, &move_gen, &zobrist),
        0
    );
}

#[test]
fn reduced_quiescence_searches_quiet_check_evasions() {
    let move_gen = MoveGen::init();
    let zobrist = Zobrist::default();
    let pos = Position::from("r6k/8/8/8/8/8/8/K7 w - - 0 1", &move_gen, &zobrist).unwrap();
    assert!(pos.board.nof_checkers > 0);
    assert!(!pos.legal_search_moves().is_empty());
    assert!(pos
        .legal_search_moves()
        .iter()
        .all(|&mov| !_move::is_eating(mov) && !_move::is_promotion(mov)));
    let mut searcher = Searcher::from(&pos, false);
    let move_end = Searcher::reduced_move_end(&mut searcher.positions[0], -1, true);
    assert_eq!(move_end, None);

    assert!(search(&mut searcher, -1, move_end, BETA_INIT, &move_gen, &zobrist) > -MATE_EVAL);
    assert!(searcher.search_data[0].sel_depth > 0);
}

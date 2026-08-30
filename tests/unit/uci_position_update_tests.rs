use super::*;

const KINGS_ONLY_FEN: &str = "4k3/8/8/8/8/8/8/4K3 w - - 0 1";
const HISTORY_MARKER: i16 = 321;

fn startpos_command(moves: &[&str]) -> PositionCommand {
    PositionCommand::new(None, moves.iter().map(|mov| (*mov).to_owned()).collect())
}

fn assert_same_position(actual: &CpuGame, expected: &CpuGame) {
    assert_eq!(actual.position.zhash, expected.position.zhash);
    assert!(
        actual
            .position
            .board
            .eq(&expected.position.board, &actual.move_gen)
    );
}

#[test]
fn optimally_syncs_every_appended_move() {
    let mut game = CpuGame::default();
    game.searcher.search_data[0].history_table[0] = HISTORY_MARKER;

    let initial = startpos_command(&[]);
    let after_one_ply = startpos_command(&["e2e4"]);
    assert_eq!(
        update_position(&mut game, &initial, &after_one_ply).unwrap(),
        PositionUpdateMethod::Synced
    );
    assert_eq!(
        game.searcher.search_data[0].history_table[0],
        HISTORY_MARKER,
        "incremental sync should preserve search history"
    );

    let after_three_plies = startpos_command(&["e2e4", "e7e5", "g1f3"]);
    assert_eq!(
        update_position(&mut game, &after_one_ply, &after_three_plies).unwrap(),
        PositionUpdateMethod::Synced
    );
    assert_eq!(
        game.searcher.search_data[0].history_table[0],
        HISTORY_MARKER,
        "two-ply incremental sync should preserve search history"
    );

    let mut imported = CpuGame::default();
    imported
        .import_position(after_three_plies.fen.as_str(), after_three_plies.moves.clone())
        .unwrap();
    assert_same_position(&game, &imported);
}

#[test]
fn imports_when_move_history_is_not_an_extension() {
    let previous = startpos_command(&["e2e4"]);
    let divergent = startpos_command(&["d2d4"]);
    let mut game = CpuGame::default();
    game.import_position(previous.fen.as_str(), previous.moves.clone())
        .unwrap();
    game.searcher.search_data[0].history_table[0] = HISTORY_MARKER;

    assert_eq!(
        update_position(&mut game, &previous, &divergent).unwrap(),
        PositionUpdateMethod::Imported
    );
    assert_eq!(
        game.searcher.search_data[0].history_table[0], 0,
        "full import should reset search history"
    );

    let mut imported = CpuGame::default();
    imported
        .import_position(divergent.fen.as_str(), divergent.moves.clone())
        .unwrap();
    assert_same_position(&game, &imported);
}

#[test]
fn imports_when_base_fen_changes() {
    let previous = startpos_command(&["e2e4"]);
    let replacement = PositionCommand::new(Some(KINGS_ONLY_FEN.to_owned()), vec![]);
    let mut game = CpuGame::default();
    game.import_position(previous.fen.as_str(), previous.moves.clone())
        .unwrap();

    assert_eq!(
        update_position(&mut game, &previous, &replacement).unwrap(),
        PositionUpdateMethod::Imported
    );

    let mut imported = CpuGame::default();
    imported
        .import_position(replacement.fen.as_str(), replacement.moves.clone())
        .unwrap();
    assert_same_position(&game, &imported);
}

#[test]
fn falls_back_to_import_when_incremental_sync_is_not_possible() {
    let previous = startpos_command(&["e2e4"]);
    let next = startpos_command(&["e2e4", "e7e5"]);
    let mut desynced_game = CpuGame::default();
    desynced_game.searcher.search_data[0].history_table[0] = HISTORY_MARKER;

    assert_eq!(
        update_position(&mut desynced_game, &previous, &next).unwrap(),
        PositionUpdateMethod::Imported
    );
    assert_eq!(
        desynced_game.searcher.search_data[0].history_table[0], 0,
        "fallback import should reset partially reusable search state"
    );

    let mut imported = CpuGame::default();
    imported
        .import_position(next.fen.as_str(), next.moves.clone())
        .unwrap();
    assert_same_position(&desynced_game, &imported);
}

#[test]
fn applies_set_option_after_reclaiming_engine_from_search_thread() {
    let mut cpu_game = None;
    let mut active_search_thread = Some(std::thread::spawn(|| Box::new(CpuGame::default())));
    let command = SetOptionCommand {
        name: "Hash".to_owned(),
        value: Some("1".to_owned()),
    };

    apply_set_option_to_engine(&command, &mut cpu_game, &mut active_search_thread);

    assert!(active_search_thread.is_none());
    let cpu_game = cpu_game.expect("engine should be reclaimed from the search thread");
    assert_eq!(
        cpu_game.searcher.tt.nof_clusters,
        (1024 * 1024) / std::mem::size_of::<crate::search::tt::TTCluster>()
    );
}

use super::{
    command_listener::{apply_set_option_to_engine, parse_command, uci_identification_lines},
    uci_command::{ArbiterCommand, GoCommand, SetOptionCommand},
};
use crate::{
    game::cpu_game::CpuGame,
    search::{searcher::MAX_SEARCH_DEPTH, tt::TTCluster},
};

const HASH_OPTION_LINE: &str = "option name Hash type spin default 16 min 1 max 32768";

fn parse_go(line: &str) -> Option<GoCommand> {
    match parse_command(line) {
        Some(ArbiterCommand::Go(command)) => Some(command),
        _ => None,
    }
}

fn parse_set_option(line: &str) -> Option<SetOptionCommand> {
    match parse_command(line) {
        Some(ArbiterCommand::SetOption(command)) => Some(command),
        _ => None,
    }
}

fn set_option(name: &str, value: Option<&str>) -> SetOptionCommand {
    SetOptionCommand {
        name: name.to_owned(),
        value: value.map(str::to_owned),
    }
}

fn apply_set_option(cpu_game: &mut Option<Box<CpuGame>>, command: &SetOptionCommand) {
    let mut active_search_thread = None;
    apply_set_option_to_engine(command, cpu_game, &mut active_search_thread);
}

#[test]
fn advertises_hash_option_before_uciok() {
    let lines = uci_identification_lines();
    let id_name_index = lines
        .iter()
        .position(|line| line.starts_with("id name "))
        .expect("engine name should be advertised");
    let hash_option_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (line == HASH_OPTION_LINE).then_some(index))
        .collect();
    let uciok_index = lines
        .iter()
        .position(|line| line == "uciok")
        .expect("uciok should be emitted");

    assert_eq!(hash_option_indices.len(), 1);
    assert!(id_name_index < hash_option_indices[0]);
    assert!(hash_option_indices[0] < uciok_index);
    assert_eq!(uciok_index, lines.len() - 1);
}

#[test]
fn parses_set_option_name_and_value() {
    let command = parse_set_option("setoption name Hash value 64").unwrap();

    assert_eq!(command.name, "Hash");
    assert_eq!(command.value.as_deref(), Some("64"));
}

#[test]
fn parses_spaced_set_option_names_and_values() {
    let command =
        parse_set_option("  setoption\tname Future Option   value multi word value  ").unwrap();

    assert_eq!(command.name, "Future Option");
    assert_eq!(command.value.as_deref(), Some("multi word value"));
}

#[test]
fn parses_set_option_without_a_value() {
    let command = parse_set_option("setoption name Clear Hash").unwrap();

    assert_eq!(command.name, "Clear Hash");
    assert_eq!(command.value, None);
}

#[test]
fn rejects_malformed_set_option_commands() {
    for line in [
        "setoption",
        "setoption Hash value 1",
        "setoption name",
        "setoption name value 1",
    ] {
        assert!(parse_set_option(line).is_none(), "{line} should be invalid");
    }
}

#[test]
fn hash_option_is_case_insensitive_and_resizes_in_mb() {
    let mut cpu_game = Some(Box::new(CpuGame::default()));

    apply_set_option(&mut cpu_game, &set_option("hAsH", Some("1")));

    assert_eq!(
        cpu_game.unwrap().searcher.tt.nof_clusters,
        (1024 * 1024) / std::mem::size_of::<TTCluster>()
    );
}

#[test]
fn invalid_and_unknown_options_leave_hash_unchanged() {
    let mut cpu_game = Some(Box::new(CpuGame::default()));
    let original_cluster_count = cpu_game.as_ref().unwrap().searcher.tt.nof_clusters;

    for command in [
        set_option("Hash", Some("0")),
        set_option("Hash", Some("32769")),
        set_option("Hash", Some("not-a-number")),
        set_option("Hash", None),
        set_option("Future Option", Some("1")),
    ] {
        apply_set_option(&mut cpu_game, &command);
        assert_eq!(
            cpu_game.as_ref().unwrap().searcher.tt.nof_clusters,
            original_cluster_count
        );
    }
}

#[test]
fn parses_display_command() {
    assert!(matches!(parse_command("d"), Some(ArbiterCommand::Display)));
}

#[test]
fn parses_static_depth() {
    let command = parse_go("go depth 7").expect("valid go command");

    assert_eq!(command.depth, Some(7));
    assert!(command.is_valid());
}

#[test]
fn parses_pondered_static_depth() {
    let command = parse_go("go ponder depth 7").expect("valid go command");

    assert!(command.ponder);
    assert_eq!(command.depth, Some(7));
    assert!(command.is_valid());
}

#[test]
fn rejects_missing_or_invalid_go_values_without_panicking() {
    for line in [
        "go depth",
        "go depth nope",
        "go movetime",
        "go wtime 100 btime",
    ] {
        assert!(parse_command(line).is_none(), "{line} should be invalid");
    }
}

#[test]
fn rejects_depth_above_search_capacity() {
    let command = parse_go(&format!("go depth {}", MAX_SEARCH_DEPTH + 1))
        .expect("syntactically valid go command");

    assert!(!command.is_valid());
}

use super::{
    command_listener::parse_command,
    uci_command::{ArbiterCommand, GoCommand},
};
use crate::search::searcher::MAX_SEARCH_DEPTH;

fn parse_go(line: &str) -> Option<GoCommand> {
    match parse_command(line) {
        Some(ArbiterCommand::Go(command)) => Some(command),
        _ => None,
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

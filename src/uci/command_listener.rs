use std::{
    io::*,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc, OnceLock,
    },
    thread::{self, JoinHandle},
    time::Instant,
};

use std::sync::Mutex;

use crate::{
    game::cpu_game::CpuGame,
    repr::{
        _move::{self, NULL_MOVE},
        types::{VERSION, WHITE},
    },
    search::search_config::{SearchMode, TimeBudget},
    uci::uci_command::{
        resolve_uci_option, ArbiterCommand, GoCommand, PositionCommand, SetOptionCommand,
        UciOptionId, UciOptionValue, UCI_OPTIONS,
    },
    utils::fen_tool::is_valid_fen,
};

#[derive(Debug, PartialEq, Eq)]
enum PositionUpdateMethod {
    Synced,
    Imported,
}

fn update_position(
    cpu_game: &mut CpuGame,
    previous: &PositionCommand,
    next: &PositionCommand,
) -> std::result::Result<PositionUpdateMethod, String> {
    let (preceeds, offset) = previous.preceeds(next);
    let mut sync_error = None;

    if preceeds {
        for mov in &next.moves[next.moves.len() - offset..] {
            if let Err(err) = cpu_game.sync_new_move(mov) {
                sync_error = Some(err);
                break;
            }
        }

        if sync_error.is_none() {
            return Ok(PositionUpdateMethod::Synced);
        }
    }

    cpu_game
        .import_position(next.fen.as_str(), next.moves.clone())
        .map_err(|import_error| match sync_error {
            Some(sync_error) => format!(
                "incremental sync failed ({sync_error}); full import failed ({import_error})"
            ),
            None => import_error,
        })?;
    Ok(PositionUpdateMethod::Imported)
}

pub(super) fn uci_identification_lines() -> Vec<String> {
    let mut lines = Vec::with_capacity(UCI_OPTIONS.len() + 2);
    lines.push(format!("id name Rusty {}", VERSION));
    lines.push("id author Otto Freund".to_owned());
    lines.extend(UCI_OPTIONS.iter().map(ToString::to_string));
    lines.push("uciok".to_owned());
    lines
}

fn apply_resolved_option(cpu_game: &mut CpuGame, option: (UciOptionId, UciOptionValue)) {
    match option {
        (UciOptionId::Hash, UciOptionValue::Spin(size_mb)) => {
            let size_mb = u32::try_from(size_mb).expect("validated Hash size should fit in u32");
            cpu_game.searcher.tt.resize(size_mb);
        }
        (UciOptionId::Threads, UciOptionValue::Spin(t)) => {
            if t != 1 {
                println!("info string Rusty only supports 1 thread, ignoring Threads option");
            }
        }
        (UciOptionId::MoveOverhead, UciOptionValue::Spin(overhead)) => {
            cpu_game.searcher.search_config.move_overhead_ms = overhead as u64;
        }
        (UciOptionId::Ponder, UciOptionValue::Check) => {
            // The GUI enables pondering per move with `go ponder`.
        }
        _ => unreachable!("validated UCI option type"),
    }
}

#[derive(Clone)]
enum SearchSignal {
    Normal,
    Infinite,
    Ponder(Arc<OnceLock<Instant>>),
}

impl SearchSignal {
    fn withhold_bestmove(&self) -> bool {
        match self {
            Self::Normal => false,
            Self::Infinite => true,
            Self::Ponder(start) => start.get().is_none(),
        }
    }
}

fn start_uci_search(
    mut game: Box<CpuGame>,
    command: &GoCommand,
    kill_switch: Arc<AtomicBool>,
) -> (JoinHandle<Box<CpuGame>>, SearchSignal) {
    let config = &mut game.searcher.search_config;
    let white_to_move = game.searcher.positions[0].board.turn == WHITE;
    config.search_mode = search_mode(command, white_to_move, config.move_overhead_ms);
    let signal = if command.ponder {
        SearchSignal::Ponder(Arc::new(OnceLock::new()))
    } else if command.infinite {
        SearchSignal::Infinite
    } else {
        SearchSignal::Normal
    };
    config.ponder_start = match &signal {
        SearchSignal::Ponder(start) => Some(start.clone()),
        _ => None,
    };

    let worker_signal = signal.clone();
    kill_switch.store(false, Relaxed);
    let handle = thread::Builder::new()
        .name("uci-search-thread".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            game.searcher
                .start_search(&game.move_gen, &game.zobrist, Some(kill_switch.clone()));
            while worker_signal.withhold_bestmove() && !kill_switch.load(Relaxed) {
                thread::park();
            }

            let best_move = game.searcher.collect_best_move().unwrap_or_else(|| {
                game.searcher.positions[0]
                    .legal_moves()
                    .first()
                    .copied()
                    .unwrap_or(NULL_MOVE)
            });
            let best_move = _move::to_string(best_move, true);
            if let Some(ponder_move) = game.searcher.collect_ponder_move() {
                println!(
                    "bestmove {best_move} ponder {}",
                    _move::to_string(ponder_move, true)
                );
            } else {
                println!("bestmove {best_move}");
            }
            game
        })
        .unwrap();
    (handle, signal)
}

fn stop_search(kill_switch: &AtomicBool, active_search: Option<&JoinHandle<Box<CpuGame>>>) {
    if let Some(handle) = active_search {
        kill_switch.store(true, Relaxed);
        handle.thread().unpark();
    }
}

fn reclaim_cpu_game(
    cpu_game: &mut Option<Box<CpuGame>>,
    active_search_thread: &mut Option<JoinHandle<Box<CpuGame>>>,
) {
    if let Some(handle) = active_search_thread.take() {
        *cpu_game = Some(handle.join().unwrap());
    }
}

pub(super) fn apply_set_option_to_engine(
    command: &SetOptionCommand,
    cpu_game: &mut Option<Box<CpuGame>>,
    active_search_thread: &mut Option<JoinHandle<Box<CpuGame>>>,
) {
    let Some(option) = resolve_uci_option(command) else {
        return;
    };

    reclaim_cpu_game(cpu_game, active_search_thread);
    if let Some(cpu_game) = cpu_game.as_deref_mut() {
        apply_resolved_option(cpu_game, option);
    }
}

pub async fn listen(cpu_game: CpuGame) {
    let stdin = std::io::stdin();
    let mut display_board = cpu_game.position.board.clone();
    let mut active_search_thread = None;
    let mut search_signal = SearchSignal::Normal;
    let mut cpu_game: Option<Box<CpuGame>> = Some(Box::new(cpu_game));
    let search_kill_switch: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let last_pos_command: Arc<Mutex<PositionCommand>> =
        Arc::new(Mutex::new(PositionCommand::new(None, vec![])));

    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let command = parse_command(&line);
        match command {
            Some(c) => {
                match c {
                    ArbiterCommand::UCI => {
                        for line in uci_identification_lines() {
                            println!("{line}");
                        }
                    }
                    ArbiterCommand::Display => {
                        println!("{}", display_board.to_string());
                    }
                    ArbiterCommand::IsReady => {
                        println!("readyok");
                    }
                    ArbiterCommand::SetOption(command) => {
                        if search_signal.withhold_bestmove() && resolve_uci_option(&command).is_some() {
                            stop_search(&search_kill_switch, active_search_thread.as_ref());
                        }
                        apply_set_option_to_engine(&command, &mut cpu_game, &mut active_search_thread);
                    }
                    ArbiterCommand::UCINewGame => {
                        stop_search(&search_kill_switch, active_search_thread.as_ref());
                        reclaim_cpu_game(&mut cpu_game, &mut active_search_thread);
                        let cpu_g: &mut Box<CpuGame> = cpu_game.as_mut().unwrap();
                        cpu_g.searcher.reset();
                    }
                    ArbiterCommand::Go(gc) if gc.is_valid() => {
                        stop_search(&search_kill_switch, active_search_thread.as_ref());
                        reclaim_cpu_game(&mut cpu_game, &mut active_search_thread);

                        let (handle, signal) =
                            start_uci_search(cpu_game.take().unwrap(), &gc, search_kill_switch.clone());
                        active_search_thread = Some(handle);
                        search_signal = signal;
                    }
                    ArbiterCommand::Go(_) => {
                        println!("info string Invalid go command");
                    }
                    ArbiterCommand::PonderHit => {
                        if let (SearchSignal::Ponder(start), Some(handle)) =
                            (&search_signal, active_search_thread.as_ref())
                        {
                            let _ = start.set(Instant::now());
                            handle.thread().unpark();
                        }
                    }
                    ArbiterCommand::Position(pc) => {
                        stop_search(&search_kill_switch, active_search_thread.as_ref());
                        reclaim_cpu_game(&mut cpu_game, &mut active_search_thread);

                        let cpu_g: &mut CpuGame = cpu_game.as_mut().unwrap();
                        match last_pos_command.try_lock() {
                            Ok(mut p) => {
                                let previous: PositionCommand = (*p).clone();
                                match update_position(cpu_g, &previous, &pc) {
                                    Ok(_) => {
                                        display_board = cpu_g.position.board.clone();
                                        *p = pc.clone();
                                    }
                                    Err(err) => {
                                        println!("info string Error updating position: {}", err);
                                    }
                                }
                            }
                            Err(_) => {
                                println!("info string Multiple position commands active, aborting");
                            }
                        }
                    }
                    ArbiterCommand::Quit => {
                        stop_search(&search_kill_switch, active_search_thread.as_ref());
                        reclaim_cpu_game(&mut cpu_game, &mut active_search_thread);
                        return;
                    }
                    ArbiterCommand::Stop => {
                        stop_search(&search_kill_switch, active_search_thread.as_ref());
                    }
                }
            }
            None => match line.split_whitespace().next() {
                Some("setoption") => {}
                Some("go") => println!("info string Invalid go command"),
                _ => println!("info string Invalid command: {}", line),
            },
        }
    }
}

pub(super) fn search_mode(command: &GoCommand, white_to_move: bool, overhead: u64) -> SearchMode {
    if let Some(movetime) = command.movetime {
        SearchMode::StaticTime(movetime.saturating_sub(overhead))
    } else if let Some(depth) = command.depth {
        SearchMode::StaticDepth(depth)
    } else if let (Some(white_time), Some(black_time)) = (command.wtime, command.btime) {
        let (remaining, increment) = if white_to_move {
            (white_time, command.winc.unwrap_or(0))
        } else {
            (black_time, command.binc.unwrap_or(0))
        };
        SearchMode::Clock(TimeBudget::new(
            remaining,
            increment,
            command.moves_to_go,
            overhead,
        ))
    } else {
        SearchMode::Infinite
    }
}

pub(super) fn parse_command(line: &str) -> Option<ArbiterCommand> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    match parts[0] {
        "uci" => Some(ArbiterCommand::UCI),
        "d" => Some(ArbiterCommand::Display),
        "isready" => Some(ArbiterCommand::IsReady),
        "setoption" => parse_set_option_command(&parts).map(ArbiterCommand::SetOption),
        "position" if !is_invalid_pos_command(&parts) => {
            let moves_idx: Option<usize> = parts.iter().position(|&x| x == "moves");
            let moves: Vec<String> = match moves_idx {
                Some(idx) => parts[idx + 1..]
                    .to_vec()
                    .into_iter()
                    .map(|s| s.into())
                    .collect(),
                None => vec![],
            };

            if parts[1] == "startpos" {
                return Some(ArbiterCommand::Position(PositionCommand::new(None, moves)));
            } else {
                // "fen"
                let fen_str: String;
                if moves_idx.is_some() {
                    fen_str = parts[2..moves_idx.unwrap()].join(" ");
                } else {
                    fen_str = parts[2..].join(" ");
                }
                if is_valid_fen(&fen_str) {
                    return Some(ArbiterCommand::Position(PositionCommand::new(
                        Some(fen_str),
                        moves,
                    )));
                } else {
                    return None;
                }
            }
        }
        "go" => parse_go_command(&parts).map(ArbiterCommand::Go),
        "ucinewgame" => Some(ArbiterCommand::UCINewGame),
        "ponderhit" => Some(ArbiterCommand::PonderHit),
        "stop" => Some(ArbiterCommand::Stop),
        "quit" => Some(ArbiterCommand::Quit),
        _ => None,
    }
}

fn parse_set_option_command(parts: &[&str]) -> Option<SetOptionCommand> {
    if !parts.get(1)?.eq_ignore_ascii_case("name") {
        return None;
    }

    let value_idx = parts
        .iter()
        .enumerate()
        .skip(2)
        .find_map(|(idx, part)| part.eq_ignore_ascii_case("value").then_some(idx));
    let name_end = value_idx.unwrap_or(parts.len());
    if name_end == 2 {
        return None;
    }

    Some(SetOptionCommand {
        name: parts[2..name_end].join(" "),
        value: value_idx.map(|idx| parts[idx + 1..].join(" ")),
    })
}

fn parse_go_command(parts: &[&str]) -> Option<GoCommand> {
    let ponder = parts.contains(&"ponder");

    if let Some(movetime) = parse_go_value(parts, "movetime")? {
        return Some(GoCommand::new_movetime_tc(ponder, movetime));
    }
    if let Some(depth) = parse_go_value(parts, "depth")? {
        return Some(GoCommand::new_depth_tc(ponder, depth));
    }

    Some(GoCommand {
        ponder,
        wtime: parse_go_value(parts, "wtime")?,
        btime: parse_go_value(parts, "btime")?,
        winc: parse_go_value(parts, "winc")?,
        binc: parse_go_value(parts, "binc")?,
        movetime: None,
        depth: None,
        moves_to_go: parse_go_value(parts, "movestogo")?,
        infinite: parts.contains(&"infinite") || parts.len() == 1 || (ponder && parts.len() == 2),
    })
}

fn parse_go_value<T: FromStr>(parts: &[&str], name: &str) -> Option<Option<T>> {
    let Some(idx) = parts.iter().position(|&part| part == name) else {
        return Some(None);
    };
    let value = parts.get(idx + 1)?.parse::<T>().ok()?;
    Some(Some(value))
}

/// Assumes that possible FEN and moves are valid, only checks for correct command structure
fn is_invalid_pos_command(parts: &Vec<&str>) -> bool {
    if parts.len() < 2 {
        return true;
    }
    if parts[1] != "startpos" && parts[1] != "fen" {
        return true;
    }

    if parts[1] == "fen" && parts.len() < 3 {
        return true;
    }

    if parts[1] == "startpos" && parts.len() > 2 && parts[2] != "moves" {
        return true;
    }

    return false;
}

#[cfg(test)]
#[path = "../../tests/unit/uci_position_update_tests.rs"]
mod position_update_tests;

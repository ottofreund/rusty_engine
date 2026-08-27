use std::{
    io::*,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc,
    },
};

use iced::futures::lock::Mutex;

use crate::{
    game::cpu_game::CpuGame,
    repr::{
        _move::{self, NULL_MOVE}, types::WHITE,
    },
    search::search_config::SearchMode,
    uci::uci_command::{_Option::Ponder, ArbiterCommand, GoCommand, PositionCommand},
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

pub async fn listen(cpu_game: CpuGame) {
    let stdin = std::io::stdin();
    let mut display_board = cpu_game.position.board.clone();
    let mut active_search_thread: Option<std::thread::JoinHandle<Box<CpuGame>>> = None;
    let mut cpu_game: Option<Box<CpuGame>> = Some(Box::new(cpu_game));
    let search_kill_switch: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let last_pos_command: Arc<Mutex<PositionCommand>> = Arc::new(Mutex::new(PositionCommand::new(None, vec![])));
  
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let command = parse_command(&line);
        match command {
            Some(c) => {
                match c {
                    ArbiterCommand::UCI => {
                        println!("id name Rusty");
                        //println!("option name Ponder type check default true");
                        println!("uciok");
                    }
                    ArbiterCommand::Display => {
                        println!("{}", display_board.to_string());
                    }
                    ArbiterCommand::IsReady => {
                        println!("readyok");
                    }
                    ArbiterCommand::SetOption(o) => {
                        match o {
                            Ponder(_) => {
                                //can ignore safely, ponder if get "go ponder" else don't, no need for engine to know if ponder is enabled or not
                            }
                        }
                    }
                    ArbiterCommand::UCINewGame => {
                        if let Some(handle) = active_search_thread.take() {
                            search_kill_switch.store(true, Relaxed);
                            cpu_game = Some(handle.join().unwrap());
                        }
                        let cpu_g: &mut Box<CpuGame> = cpu_game.as_mut().unwrap();
                        cpu_g.searcher.reset();
                    }
                    ArbiterCommand::Go(gc) if gc.is_valid() => {
                        //TODO add ponder case
                        //join possible previous search thread before starting a new one
                        if let Some(handle) = active_search_thread.take() {
                            cpu_game = Some(handle.join().unwrap());
                        }

                        let mut game: Box<CpuGame> = cpu_game.take().unwrap();
                        let kill_switch_clone = search_kill_switch.clone();
                        kill_switch_clone.store(false, Relaxed);
                        active_search_thread = Some(std::thread::Builder::new()
                                .name("uci-search-thread".into())
                                .stack_size(32 * 1024 * 1024)
                                .spawn(move || {
                            if let Some(movetime) = gc.movetime {
                                game.searcher.search_config.search_mode =
                                    SearchMode::static_time_with_margin(movetime);
                            } else if let (Some(wtime), Some(btime)) = (gc.wtime, gc.btime) {
                                game.searcher.search_config.search_mode =
                                    SearchMode::time_control_with_margin(
                                        wtime,
                                        btime,
                                        gc.winc.unwrap_or(0),
                                        gc.binc.unwrap_or(0),
                                        game.searcher.positions[0].board.turn == WHITE,
                                    );
                            } else if let Some(depth) = gc.depth {
                                game.searcher.search_config.search_mode =
                                    SearchMode::StaticDepth(depth);
                            }
                            game.searcher.start_search(
                                &game.move_gen,
                                &game.zobrist,
                                Some(kill_switch_clone),
                            );
                            let best_move: u32 = game.searcher.collect_best_move().unwrap_or(NULL_MOVE);
                            match game.searcher.collect_ponder_move() {
                                Some(pm) => {
                                    println!("bestmove {} ponder {}", _move::to_string(best_move, true), _move::to_string(pm, true))
                                }
                                None => {
                                    println!("bestmove {}", _move::to_string(best_move, true))
                                }
                            }
                            return game;
                        }).unwrap());
                    }
                    ArbiterCommand::Go(_) => {
                        println!("info string Invalid go command");
                    }
                    ArbiterCommand::PonderHit => {
                        //TODO
                    }
                    ArbiterCommand::Position(pc) => {
                        match active_search_thread.take() {
                            Some(handle) => {
                                search_kill_switch.store(true, Relaxed);
                                cpu_game = Some(handle.join().unwrap());
                            }
                            None => {}
                        }

                        let cpu_g: &mut CpuGame = cpu_game.as_mut().unwrap();
                        let previous = last_pos_command.lock().await.clone();
                        match update_position(cpu_g, &previous, &pc) {
                            Ok(_) => {
                                display_board = cpu_g.position.board.clone();
                                *last_pos_command.lock().await = pc.clone();
                            }
                            Err(err) => {
                                println!("info string Error updating position: {}", err);
                            }
                        }
                    }
                    ArbiterCommand::Quit => {
                        let kill_switch_clone = search_kill_switch.clone();
                        kill_switch_clone.store(true, Relaxed);
                        active_search_thread
                            .take()
                            .map(|handle| handle.join().unwrap());
                        return;
                    }
                    ArbiterCommand::Stop => {
                        let kill_switch_clone = search_kill_switch.clone();
                        kill_switch_clone.store(true, Relaxed);
                    }
                }
            }
            None => {
                if line.split_whitespace().next() == Some("go") {
                    println!("info string Invalid go command");
                } else {
                    println!("info string Invalid command: {}", line);
                }
            }
        }
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
        "stop" => Some(ArbiterCommand::Stop),
        "quit" => Some(ArbiterCommand::Quit),
        _ => None,
    }
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

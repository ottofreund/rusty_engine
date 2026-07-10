use std::{
    io::*,
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc,
    },
};

use iced::futures::lock::Mutex;

use crate::{
    game::cpu_game::CpuGame, repr::{
        _move::{self, NULL_MOVE}, position::Position, types::WHITE,
    }, search::search_config::SearchMode, uci::uci_command::{ArbiterCommand, PositionCommand}, utils::fen_tool::is_valid_fen,
};

pub async fn listen(cpu_game: CpuGame) {
    let stdin = std::io::stdin();
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
                        //TODO SEND OPTIONS HERE, at least pondering option
                        println!("uciok");
                    }
                    ArbiterCommand::IsReady => {
                        println!("readyok");
                    }
                    ArbiterCommand::SetOption(_) => {
                        //TODO
                    }
                    ArbiterCommand::UCINewGame => {
                        //TODO
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
                            if gc.movetime.is_some() {
                                game.searcher.search_config.search_mode =
                                    SearchMode::static_time_with_margin(gc.movetime.unwrap());
                            } else {
                                game.searcher.search_config.search_mode =
                                    SearchMode::time_control_with_margin(
                                        gc.wtime.unwrap(),
                                        gc.btime.unwrap(),
                                        gc.winc.unwrap_or(0),
                                        gc.binc.unwrap_or(0),
                                        game.searcher.positions[0].board.turn == WHITE,
                                    );
                            }
                            game.searcher.start_search(
                                &game.move_gen,
                                &game.zobrist,
                                Some(kill_switch_clone),
                            );
                            let best_move: u32 = game.searcher.collect_best_move().unwrap_or(NULL_MOVE);
                            println!("bestmove {}", _move::to_string(best_move, true));
                            return game;
                        }).unwrap());
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

                        if last_pos_command.lock().await.preceeds(&pc) {
                            match cpu_g.sync_new_move(pc.moves.last().unwrap().as_str()) {
                                Ok(()) => {  
                                    *last_pos_command.lock().await = pc.clone();
                                }
                                Err(err) => {
                                    println!("Error syncing new move: {}", err);
                                }
                            }
                        } else {
                            match cpu_g.import_position(pc.fen.as_str(), pc.moves.clone()) {
                                Ok(()) => {
                                    *last_pos_command.lock().await = pc.clone();
                                }
                                Err(err) => {
                                    println!("Error importing position: {}", err);
                                }
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
                    _ => {
                        println!("Invalid arguments: {}", line);
                    }
                }
            }
            None => {
                println!("Invalid command: {}", line);
            }
        }
    }
}

fn parse_command(line: &str) -> Option<ArbiterCommand> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    match parts[0] {
        "uci" => Some(ArbiterCommand::UCI),
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
        "go" => {
            let static_move_time: Option<usize> = parts.iter().position(|&x| x == "movetime");
            if parts.len() > 2 && parts[1] == "ponder" {
                return Some(ArbiterCommand::Go(crate::uci::uci_command::GoCommand {
                    ponder: true,
                    wtime: None,
                    btime: None,
                    winc: None,
                    binc: None,
                    movetime: None,
                }));
            } else if let Some(idx) = static_move_time {
                let movetime = parts[idx + 1].parse::<u64>().unwrap();
                return Some(ArbiterCommand::Go(crate::uci::uci_command::GoCommand {
                    ponder: false,
                    wtime: None,
                    btime: None,
                    winc: None,
                    binc: None,
                    movetime: Some(movetime),
                }));
            } else {
                let wtime = parts
                    .iter()
                    .position(|&x| x == "wtime")
                    .map(|idx| parts[idx + 1].parse::<u64>().unwrap());
                let btime = parts
                    .iter()
                    .position(|&x| x == "btime")
                    .map(|idx| parts[idx + 1].parse::<u64>().unwrap());
                let winc = parts
                    .iter()
                    .position(|&x| x == "winc")
                    .map(|idx| parts[idx + 1].parse::<u64>().unwrap());
                let binc = parts
                    .iter()
                    .position(|&x| x == "binc")
                    .map(|idx| parts[idx + 1].parse::<u64>().unwrap());
                return Some(ArbiterCommand::Go(crate::uci::uci_command::GoCommand {
                    ponder: false,
                    wtime,
                    btime,
                    winc,
                    binc,
                    movetime: None,
                }));
            }
        },
        "ucinewgame" => Some(ArbiterCommand::UCINewGame),
        "stop" => Some(ArbiterCommand::Stop),
        "quit" => Some(ArbiterCommand::Quit),
        _ => None,
    }
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

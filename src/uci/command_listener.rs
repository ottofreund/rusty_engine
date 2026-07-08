use std::io::*;

use crate::{
    game::cpu_game::CpuGame,
    repr::{_move, types::WHITE},
    search::search_config::SearchMode,
    uci::uci_command::{ArbiterCommand, PositionCommand},
};

pub fn listen(cpu_game: CpuGame) {
    let stdin = std::io::stdin();
    let mut active_search_thread: Option<std::thread::JoinHandle<CpuGame>> = None;
    let mut cpu_game: Option<CpuGame> = Some(cpu_game);
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
                    ArbiterCommand::Go(g) if g.is_valid() => {
                        //join possible previous search thread before starting a new one
                        if let Some(handle) = active_search_thread.take() {
                            cpu_game = Some(handle.join().unwrap());
                        }

                        let mut game: CpuGame = cpu_game.take().unwrap();

                        active_search_thread = Some(std::thread::spawn(move || {
                            if g.movetime.is_some() {
                                game.searcher.search_config.search_mode =
                                    SearchMode::static_time_with_margin(g.movetime.unwrap());
                            } else {
                                game.searcher.search_config.search_mode =
                                    SearchMode::time_control_with_margin(
                                        g.wtime.unwrap(),
                                        g.btime.unwrap(),
                                        g.winc.unwrap_or(0),
                                        g.binc.unwrap_or(0),
                                        game.searcher.positions[0].board.turn == WHITE,
                                    );
                            }
                            game.searcher.start_search(&game.move_gen, &game.zobrist);
                            let best_move: u32 = game.searcher.collect_best_move().unwrap();
                            println!("bestmove {}", _move::to_string(best_move, true));
                            game.position.make_move(
                                best_move,
                                false,
                                false,
                                &game.move_gen,
                                &game.zobrist,
                            );
                            game.searcher.sync_new_move(&game.position, Some(best_move));
                            return game;
                        }));
                    }
                    ArbiterCommand::PonderHit => {
                        //TODO
                    }
                    ArbiterCommand::Position(_) => {
                        //TODO
                    }
                    ArbiterCommand::Quit => {
                        //terminate active search threads
                        return;
                    }
                    ArbiterCommand::Stop => {
                        //TODO
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
    match parts[0] {
        "uci" => Some(ArbiterCommand::UCI),
        "isready" => Some(ArbiterCommand::IsReady),
        "position" => {
            let moves_idx: Option<usize> = parts.iter().position(|&x| x == "moves");
            assert!(!is_invalid_pos_command(&parts, moves_idx.is_some()));
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
                return Some(ArbiterCommand::Position(PositionCommand::new(
                    Some(fen_str),
                    moves,
                )));
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
        }
        "stop" => Some(ArbiterCommand::Stop),
        "quit" => Some(ArbiterCommand::Quit),
        _ => None,
    }
}

/// Assumes that possible FEN and moves are valid, only checks for correct command structure
fn is_invalid_pos_command(parts: &Vec<&str>, contains_moves: bool) -> bool {
    if parts.len() < 2 || parts.len() > 3 {
        return true;
    }
    if parts[1] != "startpos" && parts[1] != "fen" {
        return true;
    }

    if parts[1] == "fen" && parts.len() < 3 {
        return true;
    } else if parts[1] != "startpos" {
        return true;
    }

    if parts.len() > 3 && !contains_moves {
        return true;
    }

    return false;
}

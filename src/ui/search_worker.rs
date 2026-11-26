use crate::{
    game::game::GameSearcher,
    repr::{move_gen::MoveGen, position::Position, types::WHITE},
    search::{
        eval::{MATE_BOUND, MATE_EVAL},
        search_config::SearchMode,
        searcher::Searcher,
    },
    utils::zobrist::Zobrist,
};
use iced::futures::channel::oneshot;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Instant,
};

use super::evaluation_history::Evaluation;

type SearchReply = (Option<u32>, Option<Evaluation>);

enum Command {
    Position(Box<Position>, Option<u32>),
    Search(SearchMode, Arc<AtomicBool>, oneshot::Sender<SearchReply>),
}

pub(super) struct SearchWorker {
    commands: mpsc::Sender<Command>,
    cancel: Option<Arc<AtomicBool>>,
}

impl SearchWorker {
    pub fn new(pos: &Position, move_gen: Arc<MoveGen>, zobrist: Arc<Zobrist>) -> Self {
        let pos = pos.clone();
        let (commands, receiver) = mpsc::channel();
        thread::spawn(move || {
            let mut searcher = Searcher::from(&pos, false);
            for command in receiver {
                match command {
                    Command::Position(pos, Some(mov)) => searcher.sync_new_move(&pos, Some(mov)),
                    Command::Position(pos, None) => searcher.import_position(&pos, None),
                    Command::Search(mode, cancel, reply) => {
                        if cancel.load(Ordering::Relaxed) {
                            continue;
                        }
                        let start = Instant::now();
                        searcher.search_config.search_mode = mode;
                        let white_to_move = searcher.positions[0].board.turn == WHITE;
                        let score = searcher.start_search(&move_gen, &zobrist, Some(cancel));
                        let evaluation = score.map(|score| to_evaluation(score, white_to_move));
                        println!("Search finished in {} ms", start.elapsed().as_millis());
                        let _ = reply.send((searcher.collect_best_move(), evaluation));
                    }
                }
            }
        });
        Self {
            commands,
            cancel: None,
        }
    }

    pub fn search(&mut self, mode: SearchMode) -> oneshot::Receiver<SearchReply> {
        let (reply, result) = oneshot::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel = Some(cancel.clone());
        let _ = self.commands.send(Command::Search(mode, cancel, reply));
        result
    }

    pub fn cancel(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
    }
}

fn to_evaluation(score: i16, white_to_move: bool) -> Evaluation {
    let white_score = if white_to_move { score } else { -score };
    if score.abs() >= MATE_BOUND {
        Evaluation::Mate {
            white_wins: white_score > 0,
            moves: (MATE_EVAL - score.abs() + 1) / 2,
        }
    } else {
        Evaluation::Centipawns(white_score)
    }
}

impl GameSearcher for SearchWorker {
    fn set_position(&mut self, position: &Position) {
        self.cancel();
        let _ = self
            .commands
            .send(Command::Position(Box::new(position.clone()), None));
    }

    fn move_played(&mut self, position: &Position, mov: u32) {
        self.cancel();
        let _ = self
            .commands
            .send(Command::Position(Box::new(position.clone()), Some(mov)));
    }
}

impl Drop for SearchWorker {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::game::Game;

    #[test]
    fn worker_searches_the_position_after_the_players_move() {
        let mut game = Game::with_searcher(SearchWorker::new);
        game.try_make_move(12, 28, None).unwrap();
        let (best_move, evaluation) =
            iced::futures::executor::block_on(game.searcher.search(SearchMode::StaticDepth(3)))
                .unwrap();
        assert!(game.position.legal_moves().contains(&best_move.unwrap()));
        let mut reference = Searcher::from(&game.position, false);
        reference.search_config.search_mode = SearchMode::StaticDepth(3);
        let score = reference
            .start_search(&game.move_gen, &game.zobrist, None)
            .unwrap();
        assert_eq!(evaluation, Some(Evaluation::Centipawns(-score)));
    }

    #[test]
    fn mate_score_keeps_distance_and_white_perspective() {
        assert_eq!(
            to_evaluation(MATE_EVAL - 3, false),
            Evaluation::Mate {
                white_wins: false,
                moves: 2
            },
        );
        assert_eq!(
            to_evaluation(-MATE_EVAL + 2, false),
            Evaluation::Mate {
                white_wins: true,
                moves: 1
            },
        );
    }
}

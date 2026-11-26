use iced::widget::{button, column, container, operation, scrollable, text};
use iced::{Element, Length, Task};

use super::evaluation_history::{self, Evaluation};
use super::messages::Message;
use crate::game::game::Game;
use crate::repr::{_move, board::Board, types::BLACK};

const LIST_ID: &str = "move-history";
const ROW_HEIGHT: f32 = 32.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Navigation {
    Previous,
    Next,
    Select(usize),
}

#[derive(Default)]
pub(super) struct History {
    reviewing: Option<usize>,
    first_move_offset: u32,
    engine_plies: Vec<usize>,
    pub(super) evaluations: Vec<Option<Evaluation>>,
}

impl History {
    pub fn reset(&mut self, first_move: u32) {
        self.engine_plies.clear();
        self.evaluations.clear();
        self.reviewing = None;
        self.first_move_offset = first_move.saturating_sub(1);
    }

    pub fn record_engine_move(&mut self, ply: usize) {
        self.engine_plies.push(ply);
    }

    pub fn record_evaluation(&mut self, ply: usize, score: Evaluation) {
        if ply >= self.evaluations.len() {
            self.evaluations.resize(ply + 1, None);
        }
        self.evaluations[ply] = Some(score);
    }

    pub fn last_engine_move<S>(&self, game: &Game<S>) -> Option<u32> {
        let moves = &game.position.played_moves_stack;
        let displayed_ply = self.reviewing.unwrap_or(moves.len());
        let count = self.engine_plies.partition_point(|&ply| ply <= displayed_ply);
        let &ply = self.engine_plies[..count].last()?;
        Some(moves[ply - 1])
    }

    pub fn is_live(&self) -> bool {
        self.reviewing.is_none()
    }

    pub fn board<'a, S>(&self, game: &'a Game<S>) -> &'a Board {
        match self.reviewing {
            Some(ply) => &game.board_history[ply],
            None => &game.position.board,
        }
    }

    pub fn navigate(&mut self, action: Navigation, last: usize) -> bool {
        let current = self.reviewing.unwrap_or(last);
        let next = match action {
            Navigation::Previous => current.saturating_sub(1),
            Navigation::Next => (current + 1).min(last),
            Navigation::Select(ply) => ply.min(last),
        };
        self.reviewing = (next < last).then_some(next);
        next != current
    }

    pub fn follow_selection(&self) -> Task<Message> {
        match self.reviewing {
            Some(ply) => operation::scroll_to(
                LIST_ID,
                scrollable::AbsoluteOffset {
                    x: 0.0,
                    y: (ply.saturating_sub(8) as f32) * ROW_HEIGHT,
                },
            ),
            None => operation::snap_to_end(LIST_ID),
        }
    }

    pub fn view<S>(
        &self,
        game: &Game<S>,
        thinking_label: Option<&'static str>,
        show_evaluation_history: bool,
    ) -> Element<'_, Message> {
        let moves = &game.position.played_moves_stack;
        let selected = self.reviewing.unwrap_or(moves.len());
        let entry = |ply, label: String| {
            button(text(label))
                .height(ROW_HEIGHT)
                .width(Length::Fill)
                .style(if ply == selected {
                    button::primary
                } else {
                    button::text
                })
                .on_press(Message::Navigate(Navigation::Select(ply)))
        };
        let mut list = column![entry(0, "Starting position".into())];
        let black_start = usize::from(game.board_history[0].turn == BLACK);
        let move_number = |ply: usize| {
            let half = ply - 1 + black_start;
            let number = self.first_move_offset as usize + 1 + half / 2;
            let dots = if half % 2 == 0 { "." } else { "..." };
            format!("{number}{dots}")
        };
        for (index, &mov) in moves.iter().enumerate() {
            let ply = index + 1;
            let number = move_number(ply);
            let notation = _move::to_string(mov, true);
            list = list.push(entry(ply, format!("{number}  {notation}")));
        }
        if let Some(label) = thinking_label {
            list = list.push(container(text(label)).padding([0, 10]).center_y(ROW_HEIGHT));
        }
        let mut content = column![
            text("Move history").size(22),
            scrollable(list).id(LIST_ID).height(Length::Fill),
            text("← Previous     Next →").size(14),
        ]
        .spacing(10);
        if show_evaluation_history {
            content = content.push(evaluation_history::view(
                &self.evaluations,
                selected,
                moves.len(),
                (!moves.is_empty()).then(|| move_number(moves.len())),
            ));
        }
        container(content)
            .padding(16)
            .width(280)
            .height(Length::Fill)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_follows_engine_moves_up_to_the_displayed_ply() {
        let mut game = Game::default();
        let mut history = History::default();
        let first = game.try_make_move(12, 28, None).unwrap(); // e2-e4
        history.record_engine_move(1);
        game.try_make_move(52, 36, None).unwrap(); // e7-e5
        let second = game.try_make_move(6, 21, None).unwrap(); // g1-f3
        history.record_engine_move(3);

        assert_eq!(history.last_engine_move(&game), Some(second));
        history.navigate(Navigation::Select(2), 3);
        assert_eq!(history.last_engine_move(&game), Some(first));
        history.navigate(Navigation::Select(0), 3);
        assert_eq!(history.last_engine_move(&game), None);
        history.reset(1);
        assert_eq!(history.last_engine_move(&game), None);
    }
}

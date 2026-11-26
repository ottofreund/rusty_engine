use std::time::{Duration, Instant};
use super::clock::{Clocks, TimeControl};
use crate::search::search_config::{SearchConfig, SearchMode};
use iced::{event, keyboard, Task};
use super::move_arrow::MoveArrow;
use super::history::{History, Navigation};
use iced::widget::image::Handle;

use iced::widget::{button, column, container, row, stack, text, Button, Image, TextInput, pick_list};
use iced::{Border, Color, Element, Shadow, Size, Subscription};

use iced_aw::{menu::*, Menu, MenuBar};

use iced_dialog::{button as dialog_button, dialog, Dialog};

use crate::ui::image_handle::ImageHandle;
use crate::ui::messages::Message::{ErrorAcknowledged, GameEndAcknowledged};
use crate::ui::messages::*;

use crate::game::game::Game;
use crate::repr::board::{RANKS};
use crate::repr::position::Position;
use crate::repr::{_move, bitboard, types::*};

use super::search_worker::SearchWorker;

pub fn run_fr() -> iced::Result {
    let size = Size::new(1160.0, 728.0);

    iced::application(|| AppState::default(), update, view)
        .subscription(AppState::subscription)
        .window(iced::window::Settings {
            size,
            min_size: Some(size),
            max_size: Some(size),
            resizable: false,
            ..Default::default()
        })
        .run()
}

pub struct AppState {
    selected_square: Option<u32>,
    promotion_target_square: Option<u32>,
    game: Game<SearchWorker>,
    search_id: u64,
    active_search: Option<u64>,
    thinking_frame: usize,
    history: History,
    show_evaluation_history: bool,
    image_handle: ImageHandle,
    selection_target_sqrs: Vec<u32>,
    fen_input: String,
    input_side: u32,
    user_side: u32,
    time_control: TimeControl,
    custom_time: [String; 3],
    clocks: Option<Clocks>,
    error: Option<&'static str>,
    show_promotion_dialog: bool,
    show_game_end_dialog: bool,
    game_end_dialog_acknowledged: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            game: Game::with_searcher(SearchWorker::new),
            search_id: 0,
            active_search: None,
            thinking_frame: 0,
            selected_square: None,
            promotion_target_square: None,
            history: History::default(),
            show_evaluation_history: true,
            image_handle: ImageHandle::default(),
            selection_target_sqrs: Vec::new(),
            fen_input: String::new(),
            input_side: WHITE,
            user_side: WHITE,
            time_control: TimeControl::Untimed,
            custom_time: ["1".into(), "0".into(), "0".into()],
            clocks: None,
            error: None,
            show_promotion_dialog: false,
            show_game_end_dialog: false,
            game_end_dialog_acknowledged: false,
        }
    }
}

impl AppState {
    fn start_search(&mut self, mode: SearchMode) -> Task<Message> {
        self.search_id += 1;
        let id = self.search_id;
        self.active_search = Some(id);
        self.thinking_frame = 0;
        let reply = self.game.searcher.search(mode);
        let search = Task::perform(
            async move { reply.await.unwrap_or_default() },
            move |(mov, score)| Message::SearchFinished(id, mov, score),
        );
        Task::batch([self.history.follow_selection(), search])
    }

    fn render_main_container(&self) -> Element<'_, Message, iced::Theme, iced::Renderer> {
        let mut sidebar = column![];
        if let Some(clocks) = &self.clocks {
            let now = Instant::now();
            for (side, name) in [(WHITE, "White"), (BLACK, "Black")] {
                let remaining = clocks.remaining(side, now);
                let secs = remaining.as_secs() + u64::from(remaining.subsec_nanos() > 0);
                let label = text(format!("{name}   {}:{:02}", secs / 60, secs % 60)).size(22);
                let active = clocks.active.is_some_and(|(turn, _)| turn == side);
                sidebar = sidebar.push(container(label).padding([4, 16]).width(280)
                    .style(if active { container::primary } else { container::transparent }));
            }
        }
        sidebar = sidebar.push(self.history.view(
            &self.game,
            self.thinking_label(),
            self.show_evaluation_history,
        ));
        let main_content = column![
            container(self.render_menu_bar()).padding([8, 24]),
            container(
                row![self.render_board(), sidebar]
                    .spacing(24)
                    .height(8 * SQR_SIZE),
            )
            .padding(24)
            .center(iced::Length::Fill),
        ]
        .width(iced::Length::Fill)
        .height(iced::Length::Fill);
        let promotion_dialog = dialog(self.show_promotion_dialog, main_content, pick_list(PROMOTION_OPTIONS, Some(PROMOTION_OPTIONS[0]), Message::PromotionSelected))
            .title("Promotion")
            .width(300.0)
            .height(200.0);
        let error_dialog = Dialog::with_buttons(
            self.error.is_some(),
            promotion_dialog,
            text(self.error.unwrap_or_default()),
            vec![dialog_button("OK", ErrorAcknowledged).into()],
        )
            .title("Error")
            .width(300.0)
            .height(200.0);
        let game_end_dialog = Dialog::with_buttons(
            self.show_game_end_dialog,
            error_dialog,
            text(match self.flagged() {
                Some(WHITE) => "Black wins on time".into(),
                Some(_) => "White wins on time".into(),
                None => self.game.game_state.to_string(),
            }),
            vec![dialog_button("OK", GameEndAcknowledged).into()],
        )
            .title("Game Over")
            .width(300.0)
            .height(200.0);
        return game_end_dialog.into();
    }

    fn render_board(&self) -> Element<'static, Message, iced::Theme, iced::Renderer> {
        let mut rows = column!();
        for i in (0..8).rev() {
            rows = rows.push(self.render_row(i));
        }
        let board = stack![rows];
        match self.history.last_engine_move(&self.game) {
            Some(mov) => board.push(iced::widget::canvas(MoveArrow(mov))
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)).into(),
            None => board.into(),
        }
    }
    fn render_row(&self, row_idx: i32) -> Element<'static, Message, iced::Theme, iced::Renderer> {
        let mut row = row![];
        let mut s: i32 = row_idx * 8;
        let e: i32 = s + 8;
        while s < e {
            row = row.push(self.render_square(s as u32));
            s += 1;
        }
        return row.into();
    }

    fn render_square(&self, square: u32) -> Element<'static, Message, iced::Theme, iced::Renderer> {
        let is_light = (square / 8 + square % 8) % 2 != 0;
        let is_selected = self.selected_square == Some(square);
        let is_targeted: bool =
            self.selected_square.is_some() && self.selection_target_sqrs.contains(&square);

        let bg_color = if is_selected {
            SELECTED_SQUARE_COLOR
        } else if is_light {
            LIGHT_SQR_COLOR
        } else {
            DARK_SQR_COLOR
        };
        let img_handle: Option<Handle> = self.get_img_for_square(square);

        let btn = match img_handle {
            Some(handle) => button(Image::new(handle).width(SQR_SIZE).height(SQR_SIZE)),
            None => button(text("").width(SQR_SIZE).height(SQR_SIZE).center()),
        }
        .width(SQR_SIZE)
        .height(SQR_SIZE)
        .style(move |_theme, _status| {
            let mut style = iced::widget::button::Style::default();
            style.background = Some(iced::Background::Color(bg_color));
            style
        })
        .on_press(Message::SquareClicked(square));

        let content: Element<'static, Message, iced::Theme, iced::Renderer>;
        if is_targeted {
            if self
                .game
                .position
                .board
                .is_occupied_by(square, self.game.position.board.turn ^ 1)
            {
                let handle: Handle = self.image_handle.target_circle_handle.clone();
                content = stack!(btn, Image::new(handle).width(SQR_SIZE).height(SQR_SIZE)).into();
            } else {
                let handle: Handle = self.image_handle.target_ball_handle.clone();
                content = stack!(btn, Image::new(handle).width(SQR_SIZE).height(SQR_SIZE)).into();
            }
        } else {
            content = btn.into();
        }
        container(content).width(SQR_SIZE).height(SQR_SIZE).into()
    }

    fn render_menu_bar(&self) -> Element<'_, Message, iced::Theme, iced::Renderer> {
        let white_button: Button<'_, Message, iced::Theme, iced::Renderer> = button("")
            .style(|_theme, _status| input_button_style(true, self.input_side == WHITE))
            .width(50.0)
            .height(50.0)
            .on_press(Message::InputSideWhitePressed);
        let black_button: Button<'_, Message, iced::Theme, iced::Renderer> = button("")
            .style(|_theme, _status| input_button_style(false, self.input_side == BLACK))
            .width(50.0)
            .height(50.0)
            .on_press(Message::InputSideBlackPressed);
        let mut controls = column![text("Time control")].spacing(4);
        for option in TimeControl::OPTIONS {
            controls = controls.push(iced::widget::radio(option.label(), option,
                Some(self.time_control), Message::TimeControlSelected));
        }
        if self.time_control == TimeControl::Custom {
            for (index, (label, message)) in [
                ("Base minutes", Message::CustomMinutes as fn(String) -> Message),
                ("Base seconds", Message::CustomSeconds),
                ("Increment (sec)", Message::CustomIncrement),
            ].into_iter().enumerate() {
                controls = controls.push(row![text(label).width(125),
                    TextInput::new("0", &self.custom_time[index]).on_input(message).width(72)]);
            }
        }
        let game_dd = Item::with_menu(
            text("Game"),
            Menu::new(
                [
                    Item::new(button("Search").on_press(Message::SearchStart)),
                    Item::new(button("Default Position").on_press(Message::NewDefaultPosPressed)),
                    Item::new(button("From FEN").on_press(Message::NewFenPosPressed)),
                    Item::new(
                        TextInput::new("FEN string", &self.fen_input)
                            .on_input(Message::FenContentChanged)
                            .on_paste(Message::FenContentChanged)
                            .width(200.0),
                    ),
                    Item::new(row![white_button, black_button]),
                    Item::new(controls),
                ]
                .into(),
            )
            .max_width(250.0)
            .padding(20.0)
            .spacing(10.0),
        );
        let view_menu = Item::with_menu(
            text("View"),
            Menu::new(vec![Item::new(
                iced::widget::checkbox(self.show_evaluation_history)
                    .label("Evaluation history")
                    .on_toggle(Message::EvaluationHistoryToggled),
            )])
            .width(iced::Length::Shrink)
            .padding(12.0),
        );
        MenuBar::new(vec![game_dd, view_menu]).spacing(16.0).into()
    }

    fn get_img_for_square(&self, sqr: u32) -> Option<Handle> {
        let owner: u32;
        let board = self.history.board(&self.game);
        if board.is_white_occupied(sqr) {
            owner = WHITE;
        } else if board.is_black_occupied(sqr) {
            owner = BLACK;
        } else {
            return None;
        }
        let piece_type: u32 = board.get_piece_type_at(sqr, owner);
        return Some(self.image_handle.img_handles[piece_type as usize].clone());
    }

    fn update_selection_targets(&mut self) {
        match self.selected_square {
            Some(sqr) => {
                self.selection_target_sqrs.clear();
                self.game.position.legal_moves().iter().for_each(|mov| {
                    if _move::get_init(*mov) == sqr {
                        self.selection_target_sqrs.push(_move::get_target(*mov));
                    }
                })
            }
            None => {}
        }
    }

    fn thinking_label(&self) -> Option<&'static str> {
        self.active_search.map(|_| THINKING_LABELS[self.thinking_frame])
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let keyboard = event::listen_with(|event, status, _| match (event, status) {
            (iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key), modifiers, ..
            }), event::Status::Ignored) if modifiers.is_empty() => match key {
                keyboard::key::Named::ArrowLeft => Some(Message::Navigate(Navigation::Previous)),
                keyboard::key::Named::ArrowRight => Some(Message::Navigate(Navigation::Next)),
                _ => None,
            },
            _ => None,
        });
        if self.active_search.is_some() || self.clocks.as_ref().is_some_and(|c| c.active.is_some()) {
            let ticks = iced::time::every(Duration::from_millis(400))
                .map(|_| Message::ThinkingTick);
            Subscription::batch([keyboard, ticks])
        } else {
            keyboard
        }
    }

    pub fn reset_state_inputs(&mut self) {
        self.fen_input = String::new();
        self.selected_square = None;
        self.promotion_target_square = None;
        self.selection_target_sqrs.clear();
    }

    pub fn is_cpu_turn(&self) -> bool {
        return self.game.position.board.turn != self.user_side;
    }

    fn reset_game_end_dialog(&mut self) {
        self.show_game_end_dialog = false;
        self.game_end_dialog_acknowledged = false;
    }

    fn flagged(&self) -> Option<u32> {
        self.clocks.as_ref().and_then(Clocks::flagged)
    }

    fn is_over(&self) -> bool {
        self.game.is_over() || self.flagged().is_some()
    }

    fn sync_game_end_dialog(&mut self) {
        self.show_game_end_dialog =
            self.is_over() && !self.game_end_dialog_acknowledged;
    }
}

pub fn update(state: &mut AppState, msg: Message) -> Task<Message> {
    update_at(state, msg, Instant::now)
}

fn update_at(state: &mut AppState, msg: Message, time: impl Fn() -> Instant) -> Task<Message> {
    let now = time();
    if state.clocks.as_mut().is_some_and(|clocks| clocks.check_flag(now)) {
        state.game.searcher.cancel();
        state.active_search = None;
        state.selected_square = None;
        state.selection_target_sqrs.clear();
        state.promotion_target_square = None;
        state.show_promotion_dialog = false;
        state.sync_game_end_dialog();
    }
    let previous_length = state.game.position.played_moves_stack.len();
    match msg {
        Message::EvaluationHistoryToggled(visible) => {
            state.show_evaluation_history = visible;
            return Task::none();
        }
        Message::Navigate(action) => {
            if state.show_promotion_dialog || state.error.is_some() || state.show_game_end_dialog {
                return Task::none();
            }
            if state.history.navigate(action, previous_length) {
                state.selected_square = None;
                state.selection_target_sqrs.clear();
                if !matches!(action, Navigation::Select(_)) {
                    return state.history.follow_selection();
                }
            }
            return Task::none();
        }
        Message::SquareClicked(_) | Message::SearchStart if !state.history.is_live() => {
            return Task::none();
        }
        Message::SquareClicked(sqr) => {
            if !state.is_over() && !state.is_cpu_turn() && !state.show_promotion_dialog {
                match state.selected_square {
                    Some(selected_sqr) => {
                        if selected_sqr == sqr {
                            //unselect
                            state.selected_square = None;
                        } else {
                            let moved_piece: Option<u32> = state.game.position.board.lift_piece_type_at(selected_sqr, state.game.position.board.turn);
                            match moved_piece {
                                Some(piece) => {
                                    if ((piece == W_PAWN && bitboard::contains_square(RANKS[6], selected_sqr) && state.game.position.board.turn == WHITE)  ||
                                       ( piece == B_PAWN && bitboard::contains_square(RANKS[1], selected_sqr) && state.game.position.board.turn == BLACK)) &&
                                         state.game.exists_move(selected_sqr, sqr)
                                    {
                                        state.show_promotion_dialog = true;
                                        state.promotion_target_square = Some(sqr);
                                    } else { //non-promotion
                                        match state.game.try_make_move(selected_sqr, sqr, None) {
                                            Ok(_) => {}
                                            Err(_) => {} //tried illegal move
                                        }
                                        state.reset_state_inputs();
                                    }
                                }
                                None => {}
                            }
                        }
                    }
                    None => {
                        let mover_occupied: u64;
                        if state.game.position.board.turn == WHITE {
                            mover_occupied = state.game.position.board.white_occupation;
                        } else {
                            mover_occupied = state.game.position.board.black_occupation;
                        }
                        if bitboard::contains_square(mover_occupied, sqr) {
                            state.selected_square = Some(sqr);
                            state.update_selection_targets();
                        } //else not valid selection
                    }
                }
            }
        }
        Message::FenContentChanged(new_str) => {
            state.fen_input = new_str;
        }
        Message::TimeControlSelected(control) => state.time_control = control,
        Message::CustomMinutes(value) => state.custom_time[0] = value,
        Message::CustomSeconds(value) => state.custom_time[1] = value,
        Message::CustomIncrement(value) => state.custom_time[2] = value,
        Message::NewDefaultPosPressed | Message::NewFenPosPressed => {
            let durations = match state.time_control.durations(&state.custom_time) {
                Ok(value) => value,
                Err(error) => {
                    state.error = Some(error);
                    return Task::none();
                }
            };
            let (position, first_move) = if msg == Message::NewFenPosPressed {
                let fen = state.fen_input.trim();
                match Position::from(fen, &state.game.move_gen, &state.game.zobrist) {
                    Ok(p) => (p, fen.split_whitespace().nth(5)
                        .and_then(|number| number.parse().ok()).unwrap_or(1)),
                    Err(_) => {
                        state.error = Some("Invalid FEN string.");
                        return Task::none();
                    }
                }
            } else {
                (Position::default(&state.game.move_gen, &state.game.zobrist), 1)
            };
            state.game.import_position(position);
            state.active_search = None;
            state.user_side = state.input_side;
            state.history.reset(first_move);
            state.reset_state_inputs();
            state.reset_game_end_dialog();
            state.show_promotion_dialog = false;
            state.error = None;
            let turn = (!state.game.is_over()).then_some(state.game.position.board.turn);
            state.clocks = durations.map(|(base, inc)| Clocks::new(base, inc, turn, time()));
        }
        Message::InputSideWhitePressed => {
            state.input_side = WHITE;
        }
        Message::InputSideBlackPressed => {
            state.input_side = BLACK;
        }
        Message::ErrorAcknowledged => {
            state.error = None;
        }
        Message::GameEndAcknowledged => {
            state.show_game_end_dialog = false;
            state.game_end_dialog_acknowledged = true;
        }
        Message::ThinkingTick => {
            if state.active_search.is_some() {
                state.thinking_frame = (state.thinking_frame + 1) % THINKING_LABELS.len();
            }
            return Task::none();
        }
        Message::SearchStart => {
            if !state.is_over() && state.active_search.is_none() {
                return state.start_search(SearchConfig::default().search_mode);
            }
        }
        Message::SearchFinished(id, mov, score) => {
            if state.active_search != Some(id) {
                return Task::none();
            }
            state.active_search = None;
            if let Some(score) = score {
                state.history.record_evaluation(previous_length, score);
            }
            if !state.is_cpu_turn() {
                return Task::none();
            }
            let Some(m) = mov else {
                return Task::none();
            };
            let played = state.game.try_make_move(
                _move::get_init(m),
                _move::get_target(m),
                _move::lift_promotion_piece(m),
            );
            if played.is_err() {
                return Task::none();
            }
            state.history.record_engine_move(state.game.position.played_moves_stack.len());
            println!("CPU played move: {}", _move::to_string(m, true));
        }
        Message::PromotionSelected(piece_str) => {
            if !state.show_promotion_dialog || state.is_over() {
                return Task::none();
            }
            state.show_promotion_dialog = false;

            let promotion_piece = match piece_str {
                "Queen" => if state.game.position.board.turn == WHITE { Some(W_QUEEN) } else { Some(B_QUEEN) },
                "Rook" => if state.game.position.board.turn == WHITE { Some(W_ROOK) } else { Some(B_ROOK) },
                "Bishop" => if state.game.position.board.turn == WHITE { Some(W_BISHOP) } else { Some(B_BISHOP) },
                "Knight" => if state.game.position.board.turn == WHITE { Some(W_KNIGHT) } else { Some(B_KNIGHT) },
                _ => panic!("Invalid promotion piece"),
            };
            state.game.try_make_move(state.selected_square.unwrap(), state.promotion_target_square.unwrap(), promotion_piece).unwrap();
            state.reset_state_inputs();
        }
        _ => {
            println!("Unrecognized message");
        }
    }
    let moved = previous_length < state.game.position.played_moves_stack.len();
    if moved {
        if let Some(clocks) = &mut state.clocks {
            clocks.finish_move(now, state.game.is_over());
        }
        state.active_search = None;
    }
    state.sync_game_end_dialog();
    if !state.is_over() && state.is_cpu_turn() && state.active_search.is_none() {
        let mode = state.clocks.as_ref().map_or_else(
            || SearchConfig::default().search_mode,
            |c| c.search_mode(state.game.position.board.turn, time()),
        );
        return state.start_search(mode);
    }
    if moved && state.history.is_live() {
        state.history.follow_selection()
    } else {
        Task::none()
    }
}

pub fn view(state: &AppState) -> Element<'_, Message> {
    let main_content = state.render_main_container();
    return main_content.into();
}

fn input_button_style(white: bool, selected: bool) -> iced::widget::button::Style {
    let main_color: iced::Color;
    let border_color: iced::Color;
    if white {
        main_color = Color::WHITE;
    } else {
        main_color = Color::BLACK;
    }
    if selected {
        border_color = Color::from_rgb(0.0, 1.0, 0.0);
    } else {
        border_color = Color::TRANSPARENT;
    }
    return iced::widget::button::Style {
        background: Some(iced::Background::Color(main_color)),
        border: Border {
            color: border_color,
            width: 2.0,
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
        text_color: Color::TRANSPARENT,
    };
}

const THINKING_LABELS: [&str; 3] = ["Thinking.", "Thinking..", "Thinking..."];
const LIGHT_SQR_COLOR: iced::Color = iced::Color::from_rgb(0.94, 0.90, 0.86);
const DARK_SQR_COLOR: iced::Color = iced::Color::from_rgb(0.47, 0.40, 0.30);
const SELECTED_SQUARE_COLOR: iced::Color = iced::Color::from_rgb(1.0, 1.0, 0.0);
const SQR_SIZE: u32 = 80;
const PROMOTION_OPTIONS: [&str; 4] = ["Queen", "Rook", "Bishop", "Knight"];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::game_state::GameState;

    const STALEMATE_FEN: &str = "k7/2Q5/2K5/8/8/8/8/8 b - - 0 1";

    fn terminal_state() -> AppState {
        let mut state = AppState::default();
        state.game.game_state = GameState::Stalemate;
        state
    }

    #[test]
    fn pending_cpu_reply_keeps_history_interactive_and_ignores_old_games() {
        let mut state = AppState::default();
        let _ = update(&mut state, Message::SquareClicked(12));
        let _ = update(&mut state, Message::SquareClicked(28));
        let old_id = state.active_search.unwrap();
        assert_eq!(state.game.position.played_moves_stack.len(), 1);
        let _ = update(&mut state, Message::SquareClicked(52));
        assert_eq!(state.selected_square, None);
        let _ = update(&mut state, Message::SearchStart);
        assert_eq!(state.active_search, Some(old_id));

        let _ = update(&mut state, Message::NewDefaultPosPressed);
        let _ = update(&mut state, Message::SquareClicked(12));
        let _ = update(&mut state, Message::SquareClicked(28));
        let new_id = state.active_search.unwrap();
        assert_ne!(old_id, new_id);
        let reply = state.game.position.legal_moves()[0];
        let score = Evaluation::Centipawns(42);
        let _ = update(&mut state, Message::SearchFinished(old_id, Some(reply), Some(score)));
        assert!(state.history.evaluations.is_empty());
        assert_eq!(state.active_search, Some(new_id));
        assert_eq!(state.game.position.played_moves_stack.len(), 1);

        let _ = update(&mut state, Message::Navigate(Navigation::Previous));
        let _ = update(&mut state, Message::SearchFinished(new_id, Some(reply), Some(score)));
        assert_eq!(state.history.evaluations, [None, Some(score)]);
        assert!(state.history.board(&state.game).is_white_occupied(12));
        assert_eq!(state.game.position.played_moves_stack.len(), 2);
        let _ = update(&mut state, Message::Navigate(Navigation::Select(2)));
        assert_eq!(state.history.last_engine_move(&state.game), Some(reply));
        let _ = update(&mut state, Message::NewDefaultPosPressed);
        assert!(state.history.evaluations.is_empty());
    }

    #[test]
    fn manual_search_does_not_play_a_move() {
        let mut state = AppState::default();
        let _ = update(&mut state, Message::SearchStart);
        let id = state.active_search.unwrap();
        assert_eq!(state.thinking_label(), Some("Thinking."));
        for label in ["Thinking..", "Thinking...", "Thinking."] {
            let _ = update(&mut state, Message::ThinkingTick);
            assert_eq!(state.thinking_label(), Some(label));
            assert!(state.game.position.played_moves_stack.is_empty());
        }
        let mov = state.game.position.legal_moves()[0];
        let score = Evaluation::Centipawns(20);
        let _ = update(&mut state, Message::SearchFinished(id, Some(mov), Some(score)));
        assert_eq!(state.history.evaluations, [Some(score)]);
        let _ = update(&mut state, Message::ThinkingTick);
        assert_eq!(state.thinking_label(), None);
        assert!(state.game.position.played_moves_stack.is_empty());
    }

    #[test]
    fn browsing_moves_preserves_live_play() {
        let mut state = AppState::default();
        state.game.try_make_move(12, 28, None).unwrap(); // e2e4
        state.game.try_make_move(52, 36, None).unwrap(); // e7e5
        let live_hash = state.game.position.zhash;
        let _ = update(&mut state, Message::SquareClicked(6));
        let _ = update(&mut state, Message::Navigate(Navigation::Previous));
        assert!(state.selected_square.is_none());
        assert!(state.history.board(&state.game).is_black_occupied(52));

        let _ = update(&mut state, Message::Navigate(Navigation::Select(0)));
        let _ = update(&mut state, Message::Navigate(Navigation::Previous));
        assert!(state.history.board(&state.game).is_white_occupied(12));
        let _ = update(&mut state, Message::SquareClicked(6));
        assert!(state.selected_square.is_none());

        let _ = update(&mut state, Message::Navigate(Navigation::Next));
        assert!(state.history.board(&state.game).is_white_occupied(28));
        let _ = update(&mut state, Message::Navigate(Navigation::Select(2)));
        let _ = update(&mut state, Message::Navigate(Navigation::Next));
        assert!(state.history.is_live());
        assert_eq!(state.game.position.zhash, live_hash);
        assert_eq!(state.game.position.played_moves_stack.len(), 2);
        let _ = update(&mut state, Message::SquareClicked(6));
        assert_eq!(state.selected_square, Some(6));

        let _ = update(&mut state, Message::Navigate(Navigation::Previous));
        let _ = update(&mut state, Message::NewDefaultPosPressed);
        assert!(state.history.is_live());
        assert!(state.game.position.played_moves_stack.is_empty());
    }

    #[test]
    fn timed_moves_and_late_engine_reply_while_browsing() {
        let mut state = AppState::default();
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);
        let send = |state: &mut AppState, ms, msg| { let _ = update_at(state, msg, || at(ms)); };
        send(&mut state, 0, Message::TimeControlSelected(TimeControl::Bullet));
        send(&mut state, 0, Message::NewDefaultPosPressed);
        send(&mut state, 100, Message::SquareClicked(12));
        send(&mut state, 200, Message::SquareClicked(44)); // illegal e2-e6
        send(&mut state, 300, Message::SquareClicked(12));
        send(&mut state, 400, Message::SquareClicked(28));
        let id = state.active_search.unwrap();
        let reply = state.game.position.legal_moves()[0];
        send(&mut state, 500, Message::Navigate(Navigation::Previous));
        let clocks = state.clocks.as_ref().unwrap();
        assert_eq!(clocks.remaining(WHITE, at(500)), Duration::from_millis(60600));
        assert_eq!(clocks.remaining(BLACK, at(500)), Duration::from_millis(59900));
        let score = Evaluation::Centipawns(42);
        send(&mut state, 60400, Message::SearchFinished(id, Some(reply), Some(score)));
        assert!(state.history.evaluations.is_empty());
        assert_eq!(state.flagged(), Some(BLACK));
        assert!(state.show_game_end_dialog);
        assert!(state.active_search.is_none());
        assert_eq!(state.game.position.played_moves_stack.len(), 1);

        send(&mut state, 61000, Message::InputSideBlackPressed);
        send(&mut state, 61000, Message::NewDefaultPosPressed);
        send(&mut state, 61000, Message::SearchFinished(id, Some(reply), Some(score)));
        assert!(state.game.position.played_moves_stack.is_empty());
        let clocks = state.clocks.as_ref().unwrap();
        for side in [WHITE, BLACK] {
            assert_eq!(clocks.remaining(side, at(61000)), Duration::from_secs(60));
        }
    }

    #[test]
    fn custom_clock_validation_preserves_current_game() {
        let mut state = AppState::default();
        let now = Instant::now();
        let send = |state: &mut AppState, msg| { let _ = update_at(state, msg, || now); };
        for (option, seconds, increment) in [
            (TimeControl::Rapid, 600, 10), (TimeControl::Blitz, 180, 3),
            (TimeControl::Bullet, 60, 1),
        ] {
            send(&mut state, Message::TimeControlSelected(option));
            assert_eq!(option.durations(&state.custom_time).unwrap(),
                Some((Duration::from_secs(seconds), Duration::from_secs(increment))));
        }
        send(&mut state, Message::NewDefaultPosPressed);
        send(&mut state, Message::SquareClicked(12));
        send(&mut state, Message::SquareClicked(28));
        let pending = state.active_search;
        send(&mut state, Message::TimeControlSelected(TimeControl::Custom));
        send(&mut state, Message::CustomMinutes("0".into()));
        for seconds in ["0", "-1", "abc"] {
            send(&mut state, Message::CustomSeconds(seconds.into()));
            send(&mut state, Message::NewDefaultPosPressed);
            assert!(state.error.is_some());
            assert_eq!(state.active_search, pending);
            assert_eq!(state.game.position.played_moves_stack.len(), 1);
            assert_eq!(state.clocks.as_ref().unwrap().active, Some((BLACK, now)));
        }
        send(&mut state, Message::CustomSeconds("2".into()));
        send(&mut state, Message::CustomIncrement("-1".into()));
        send(&mut state, Message::NewFenPosPressed);
        assert!(state.error.unwrap().contains("nonnegative"));
        send(&mut state, Message::CustomIncrement("0".into()));
        send(&mut state, Message::FenContentChanged(STALEMATE_FEN.into()));
        send(&mut state, Message::NewFenPosPressed);
        let clocks = state.clocks.as_ref().unwrap();
        assert!(clocks.active.is_none());
        assert_eq!(clocks.remaining(BLACK, now), Duration::from_secs(2));
        assert!(state.show_game_end_dialog);
    }

    #[test]
    fn black_promotion_clock_runs_until_choice_or_flag() {
        let mut state = AppState::default();
        let start = Instant::now();
        let send = |state: &mut AppState, ms, msg| {
            let _ = update_at(state, msg, || start + Duration::from_millis(ms));
        };
        send(&mut state, 0, Message::InputSideBlackPressed);
        send(&mut state, 0, Message::TimeControlSelected(TimeControl::Custom));
        send(&mut state, 0, Message::CustomMinutes("0".into()));
        send(&mut state, 0, Message::CustomSeconds("1".into()));
        for (base, delay) in [(0, 1001), (2000, 999)] {
            send(&mut state, base, Message::FenContentChanged("7k/8/8/8/8/8/p7/7K b - - 0 1".into()));
            send(&mut state, base, Message::NewFenPosPressed);
            send(&mut state, base, Message::SearchStart);
            send(&mut state, base + 100, Message::SquareClicked(8));
            send(&mut state, base + 200, Message::SquareClicked(0));
            assert!(state.show_promotion_dialog);
            send(&mut state, base + delay, Message::PromotionSelected("Queen"));
            assert!(!state.show_promotion_dialog);
            if delay > 1000 {
                assert_eq!(state.flagged(), Some(BLACK));
                assert!(state.active_search.is_none());
                assert!(state.game.position.played_moves_stack.is_empty());
            } else {
                let clocks = state.clocks.as_ref().unwrap();
                assert_eq!(clocks.remaining(BLACK, start + Duration::from_secs(3)), Duration::from_millis(1));
                assert_eq!(clocks.active.unwrap().0, WHITE);
                assert_eq!(state.game.position.played_moves_stack.len(), 1);
            }
        }
    }

    #[test]
    fn post_update_sync_shows_game_end_dialog() {
        let mut state = terminal_state();

        let _ = update(&mut state, Message::InputSideWhitePressed);

        assert!(state.show_game_end_dialog);
    }

    #[test]
    fn acknowledged_game_end_dialog_stays_closed_for_current_game() {
        let mut state = terminal_state();
        let _ = update(&mut state, Message::InputSideWhitePressed);

        let _ = update(&mut state, Message::GameEndAcknowledged);
        let _ = update(&mut state, Message::InputSideBlackPressed);

        assert!(state.game_end_dialog_acknowledged);
        assert!(!state.show_game_end_dialog);
    }

    #[test]
    fn loading_default_position_clears_game_end_dialog_dismissal() {
        let mut state = terminal_state();
        let _ = update(&mut state, Message::GameEndAcknowledged);

        let _ = update(&mut state, Message::NewDefaultPosPressed);

        assert!(!state.game_end_dialog_acknowledged);
        assert!(!state.show_game_end_dialog);
    }

    #[test]
    fn loading_terminal_fen_reopens_game_end_dialog() {
        let mut state = terminal_state();
        let _ = update(&mut state, Message::GameEndAcknowledged);
        state.input_side = BLACK;
        state.fen_input = STALEMATE_FEN.to_owned();

        let _ = update(&mut state, Message::NewFenPosPressed);

        assert!(state.game.is_over());
        assert!(!state.game_end_dialog_acknowledged);
        assert!(state.show_game_end_dialog);
    }
}

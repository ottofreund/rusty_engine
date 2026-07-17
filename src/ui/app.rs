use iced::event::{self, Event};
use iced::keyboard::{self};
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

use std::time::{Duration, Instant};

pub fn run_fr() -> iced::Result {
    iced::application(|| AppState::default(), update, view)
        //.subscription(|_| AppState::subscription())
        .resizable(false)
        .window_size(Size::new(1300.0, 700.0))
        .run()
}

#[derive(Default)]
pub struct AppState {
    selected_square: Option<u32>,
    promotion_target_square: Option<u32>,
    game: Game,
    image_handle: ImageHandle,
    selection_target_sqrs: Vec<u32>,
    fen_input: String,
    input_side: u32,
    user_side: u32,
    show_error_dialog: bool,
    show_promotion_dialog: bool,
    show_game_end_dialog: bool,
    game_end_dialog_acknowledged: bool,
}

impl AppState {
    fn render_main_container(&self) -> Element<'_, Message, iced::Theme, iced::Renderer> {
        let main_content = column![
        self.render_menu_bar(),
        self.render_board(),
        ]
        .width(iced::Length::Fill)
        .height(iced::Length::Fill);
        let promotion_dialog = dialog(self.show_promotion_dialog, main_content, pick_list(PROMOTION_OPTIONS, Some(PROMOTION_OPTIONS[0]), Message::PromotionSelected))
            .title("Promotion")
            .width(300.0)
            .height(200.0);
        let error_dialog = Dialog::with_buttons(
            self.show_error_dialog,
            promotion_dialog,
            text("Invalid FEN-string"),
            vec![dialog_button("OK", ErrorAcknowledged).into()],
        )
            .title("Error")
            .width(300.0)
            .height(200.0);
        let game_end_dialog = Dialog::with_buttons(
            self.show_game_end_dialog,
            error_dialog,
            text(self.game.game_state.to_string()),
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
        return rows.into();
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
                ]
                .into(),
            )
            .max_width(200.0)
            .padding(20.0)
            .spacing(25.0),
        );
        return MenuBar::new(vec![game_dd]).into();
    }

    fn get_img_for_square(&self, sqr: u32) -> Option<Handle> {
        let owner: u32;
        if self.game.position.board.is_white_occupied(sqr) {
            owner = WHITE;
        } else if self.game.position.board.is_black_occupied(sqr) {
            owner = BLACK;
        } else {
            return None;
        }
        let piece_type: u32 = self.game.position.board.get_piece_type_at(sqr, owner);
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

    pub fn subscription() -> Subscription<Message> {
        return event::listen().map(Message::Event);
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

    fn sync_game_end_dialog(&mut self) {
        self.show_game_end_dialog =
            self.game.is_over() && !self.game_end_dialog_acknowledged;
    }
}

pub fn update(state: &mut AppState, msg: Message) {
    match msg {
        Message::SquareClicked(sqr) => {
            if !state.game.is_over() {
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
        /* Message::Event(event) => match event { DEPRECATED
          Event::Keyboard(keyboard::Event::KeyPressed {
              key: keyboard::Key::Character(c), ..
          }) => {
              if c.len() == 1 && c.contains('r') {
                  state.game.position.try_unmake_move().unwrap();
              }
          }
          _ => {}
        } */
        Message::FenContentChanged(new_str) => {
            state.fen_input = new_str;
        }
        Message::NewDefaultPosPressed => {
            state.reset_state_inputs();
            state.reset_game_end_dialog();
            state.user_side = state.input_side;
            state
                .game
                .import_position(Position::default(&state.game.move_gen, &state.game.zobrist));
        }
        Message::NewFenPosPressed => {
            match Position::from(
                &state.fen_input.trim(),
                &state.game.move_gen,
                &state.game.zobrist,
            ) {
                Ok(p) => {
                    state.reset_state_inputs();
                    state.reset_game_end_dialog();
                    state.user_side = state.input_side;
                    state.game.import_position(p);
                }
                Err(_) => {
                    state.show_error_dialog = true;
                }
            }
        }
        Message::InputSideWhitePressed => {
            state.input_side = WHITE;
        }
        Message::InputSideBlackPressed => {
            state.input_side = BLACK;
        }
        Message::ErrorAcknowledged => {
            state.show_error_dialog = false;
        }
        Message::GameEndAcknowledged => {
            state.show_game_end_dialog = false;
            state.game_end_dialog_acknowledged = true;
        }
        Message::SearchStart => {
            if !state.game.is_over() {
                let start: Instant = Instant::now();
                state
                    .game
                    .searcher
                    .start_search(&state.game.move_gen, &state.game.zobrist, None);
                let time_took: Duration = start.elapsed();
                println!("Search finished in {} ms", time_took.as_millis());
            }
        }
        Message::PromotionSelected(piece_str) => {
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
    state.sync_game_end_dialog();
    if !state.game.is_over() && state.is_cpu_turn() {
        match state.game.play_cpu_move() {
            Ok(_) => {}
            Err(_) => {}
        }
    }
}

pub fn view(state: &AppState) -> Element<Message> {
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
    fn post_update_sync_shows_game_end_dialog() {
        let mut state = terminal_state();

        update(&mut state, Message::InputSideWhitePressed);

        assert!(state.show_game_end_dialog);
    }

    #[test]
    fn acknowledged_game_end_dialog_stays_closed_for_current_game() {
        let mut state = terminal_state();
        update(&mut state, Message::InputSideWhitePressed);

        update(&mut state, Message::GameEndAcknowledged);
        update(&mut state, Message::InputSideBlackPressed);

        assert!(state.game_end_dialog_acknowledged);
        assert!(!state.show_game_end_dialog);
    }

    #[test]
    fn loading_default_position_clears_game_end_dialog_dismissal() {
        let mut state = terminal_state();
        update(&mut state, Message::GameEndAcknowledged);

        update(&mut state, Message::NewDefaultPosPressed);

        assert!(!state.game_end_dialog_acknowledged);
        assert!(!state.show_game_end_dialog);
    }

    #[test]
    fn loading_terminal_fen_reopens_game_end_dialog() {
        let mut state = terminal_state();
        update(&mut state, Message::GameEndAcknowledged);
        state.input_side = BLACK;
        state.fen_input = STALEMATE_FEN.to_owned();

        update(&mut state, Message::NewFenPosPressed);

        assert!(state.game.is_over());
        assert!(!state.game_end_dialog_acknowledged);
        assert!(state.show_game_end_dialog);
    }
}

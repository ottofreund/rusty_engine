use iced::widget::image::Handle;
use iced::widget::{Image, button, column, container, image, row, stack, text};
use iced::{Application, Element, Program, Settings, alignment, run};
use crate::repr::game::Game;
use crate::repr::board::square_to_string;
use crate::repr::{_move, bitboard, types::*};

use crate::ui::image_handle::ImageHandle;
use crate::ui::messages::*;

#[derive(Default)]
pub struct AppState {
    selected_square: Option<u32>,
    game: Game,
    image_handle: ImageHandle,
    selection_target_sqrs: Vec<u32>,
    previous_moved_src: u32,
    previous_moved_target: u32,
}

impl AppState {
    
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
        let is_targeted: bool = self.selected_square.is_some() && self.selection_target_sqrs.contains(&square);

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
            None => button(text("").width(SQR_SIZE).height(SQR_SIZE).center())
        }.width(SQR_SIZE)
         .height(SQR_SIZE)
         .style(move |_theme, _status| {
            let mut style = iced::widget::button::Style::default();
            style.background = Some(iced::Background::Color(bg_color));
            style
         })
         .on_press(Message::SquareClicked(square));
            
        let content: Element<'static, Message, iced::Theme, iced::Renderer>;
        if is_targeted {
            if self.game.board.is_occupied_by(square, self.game.board.turn.opposite()) {
                let handle: Handle = self.image_handle.target_circle_handle.clone();
                content = stack!(btn, Image::new(handle).width(SQR_SIZE).height(SQR_SIZE)).into();
            } else {
                let handle: Handle = self.image_handle.target_ball_handle.clone();
                content = stack!(btn, Image::new(handle).width(SQR_SIZE).height(SQR_SIZE)).into();
            }
        } else {
            content = btn.into();
        }
        container(content)
            .width(SQR_SIZE)
            .height(SQR_SIZE)
            .into()
    }

    fn get_img_for_square(&self, sqr: u32) -> Option<Handle> {
        let owner: Color;
        if self.game.board.is_white_occupied(sqr) {
            owner = Color::White;
        } else if self.game.board.is_black_occupied(sqr) {
            owner = Color::Black;
        } else {
            return None;
        }
        let piece_type: u32 = self.game.board.get_piece_type_at(sqr, owner);
        return Some(self.image_handle.img_handles[piece_type as usize].clone());
    }

    fn update_selection_targets(&mut self) {
        match self.selected_square {
            Some(sqr) => {
                self.selection_target_sqrs.clear();
                self.game.legal_moves.iter().for_each(|mov| {
                    if _move::get_init(*mov) == sqr {
                        self.selection_target_sqrs.push(_move::get_target(*mov));
                    }
                })
            },
            None => {}
        }
    }

}



pub fn update(state: &mut AppState, msg: Message) {
    println!("Got update!");
    println!("Selected square is none: {}", state.selected_square.is_none());
    match msg {
      Message::Reset => {
        println!("Got to reset handler");
        state.selected_square = None;
      }
      Message::SquareClicked(sqr) => {
        println!("Clicked square: {}", square_to_string(sqr));
        match state.selected_square {
            Some(selected_sqr) => {
                if selected_sqr == sqr { //unselect
                    state.selected_square = None;
                } else { //move from selected_sqr to sqr
                    match state.game.try_make_move(selected_sqr, sqr) {
                        Ok(mov) => {
                            println!("Made move {}", _move::to_string(mov));
                            state.selected_square = None;
                        },
                        Err(e) => {
                            println!("That move is illegal.");
                            state.selected_square = None;
                        }
                    }
                }
            },
            None => {
                let mover_occupied: u64;
                if state.game.board.turn.is_white() {
                    mover_occupied = state.game.board.white_occupation;
                } else {
                    mover_occupied = state.game.board.black_occupation;
                }
                if bitboard::contains_square(mover_occupied, sqr) {
                    state.selected_square = Some(sqr);
                    state.update_selection_targets();
                } //else not valid selection
            }
        }
      }
      _ => {
        println!("Unrecognized message");
      }
    }
    return;
}

pub fn view(state: &AppState) -> Element<Message> {
    return state.render_board();
}

pub fn run_fr() -> iced::Result {
    return run(update, view);
}

const LIGHT_SQR_COLOR: iced::Color = iced::Color::from_rgb(0.94, 0.90, 0.86);
const DARK_SQR_COLOR: iced::Color = iced::Color::from_rgb(0.47, 0.40, 0.30);
const SELECTED_SQUARE_COLOR: iced::Color = iced::Color::from_rgb(1.0, 1.0, 0.0);
const SQR_SIZE: u32 = 80;
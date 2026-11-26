use iced::widget::canvas::{Frame, Geometry, Path, Program};
use iced::{mouse, Color, Point, Rectangle, Renderer, Theme, Vector};

use crate::repr::_move;

pub(super) struct MoveArrow(pub u32);

impl<Message> Program<Message> for MoveArrow {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let cell = bounds.width / 8.0;
        let center = |square: u32| {
            Point::new(
                (square % 8) as f32 * cell + cell / 2.0,
                (7 - square / 8) as f32 * cell + cell / 2.0,
            )
        };
        let start = center(_move::get_init(self.0));
        let delta = center(_move::get_target(self.0)) - start;
        // Stop halfway from the entry edge (or corner) to the destination center.
        let span = delta.x.abs().max(delta.y.abs());
        let tip = delta.x.hypot(delta.y) * (1.0 - 0.25 * cell / span);
        let neck = tip - cell * 0.32;
        let shaft = cell * 0.055;
        let head = cell * 0.17;

        let mut frame = Frame::new(renderer, bounds.size());
        frame.translate(Vector::new(start.x, start.y));
        frame.rotate(delta.y.atan2(delta.x));
        let arrow = Path::new(|path| {
            path.move_to(Point::new(0.0, -shaft));
            path.line_to(Point::new(neck, -shaft));
            path.line_to(Point::new(neck, -head));
            path.line_to(Point::new(tip, 0.0));
            path.line_to(Point::new(neck, head));
            path.line_to(Point::new(neck, shaft));
            path.line_to(Point::new(0.0, shaft));
            path.close();
        });
        frame.fill(&arrow, Color::from_rgba(0.12, 0.43, 0.95, 0.78));
        vec![frame.into_geometry()]
    }
}

use iced::widget::{
    canvas::{self, Frame, Geometry, Path, Program, Stroke},
    column, text,
};
use iced::{alignment, mouse, Element, Length, Point, Rectangle, Renderer, Theme};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Evaluation {
    Centipawns(i16),
    Mate { white_wins: bool, moves: i16 },
}

impl Evaluation {
    fn label(self) -> String {
        match self {
            Self::Centipawns(score) => format!("{:+.2} pawns", f32::from(score) / 100.0),
            Self::Mate { white_wins, moves } => {
                let side = if white_wins { "White" } else { "Black" };
                format!("{side} mates in {moves}")
            }
        }
    }

    fn chart_value(self, limit: f32) -> f32 {
        match self {
            Self::Centipawns(score) => f32::from(score) / 100.0,
            Self::Mate {
                white_wins: true, ..
            } => limit,
            Self::Mate {
                white_wins: false, ..
            } => -limit,
        }
    }
}

pub(super) fn view<Message: 'static>(
    samples: &[Option<Evaluation>],
    selected: usize,
    last: usize,
    end_label: Option<String>,
) -> Element<'_, Message> {
    let evaluation = match (samples.get(selected).copied().flatten(), samples.last()) {
        (Some(score), _) => score.label(),
        (None, Some(Some(score))) if selected == last => format!("Latest · {}", score.label()),
        (None, Some(_)) => "Position not searched".into(),
        (None, None) => "Waiting for a search".into(),
    };
    column![
        text("Evaluation history").size(18),
        text(evaluation).size(14),
        canvas::Canvas::new(Chart {
            samples,
            selected,
            last,
            end_label
        })
        .width(Length::Fill)
        .height(128),
        text("Positive favors White").size(12),
    ]
    .spacing(4)
    .into()
}

struct Chart<'a> {
    samples: &'a [Option<Evaluation>],
    selected: usize,
    last: usize,
    end_label: Option<String>,
}

impl<Message> Program<Message> for Chart<'_> {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let palette = theme.extended_palette();
        // Iced 0.14's software renderer applies the canvas offset twice to the clip.
        let clip = Rectangle {
            x: -bounds.x,
            y: -bounds.y,
            width: bounds.width + bounds.x,
            height: bounds.height + bounds.y,
        };
        let mut frame = Frame::with_bounds(renderer, clip);
        let left = 36.0;
        let right = bounds.width - 8.0;
        let top = 10.0;
        let bottom = bounds.height - 23.0;
        let limit = self
            .samples
            .iter()
            .flatten()
            .filter_map(|score| match *score {
                Evaluation::Centipawns(cp) => Some(f32::from(cp).abs() / 100.0),
                Evaluation::Mate { .. } => None,
            })
            .fold(1.0, f32::max)
            .ceil();
        let x = |ply: usize| left + (right - left) * ply as f32 / self.last.max(1) as f32;
        let y = |value: f32| top + (bottom - top) * (1.0 - value / limit) / 2.0;
        let grid = Stroke::default().with_color(palette.background.strong.color);
        let label = |content, position, align_x: alignment::Horizontal, align_y| canvas::Text {
            content,
            position,
            align_x: align_x.into(),
            align_y,
            color: palette.background.base.text,
            size: 11.into(),
            ..canvas::Text::default()
        };

        for (value, tick) in [
            (limit, format!("+{limit}")),
            (0.0, "0".into()),
            (-limit, format!("-{limit}")),
        ] {
            let line_y = y(value);
            frame.stroke(
                &Path::line(Point::new(left, line_y), Point::new(right, line_y)),
                grid,
            );
            frame.fill_text(label(
                tick,
                Point::new(left - 6.0, line_y),
                alignment::Horizontal::Right,
                alignment::Vertical::Center,
            ));
        }
        frame.fill_text(label(
            "Start".into(),
            Point::new(left, bottom + 6.0),
            alignment::Horizontal::Left,
            alignment::Vertical::Top,
        ));
        if let Some(end_label) = &self.end_label {
            frame.fill_text(label(
                end_label.clone(),
                Point::new(right, bottom + 6.0),
                alignment::Horizontal::Right,
                alignment::Vertical::Top,
            ));
        }

        if !self.samples.is_empty() {
            let cursor_x = x(self.selected);
            frame.stroke(
                &Path::line(Point::new(cursor_x, top), Point::new(cursor_x, bottom)),
                Stroke::default().with_color(palette.primary.weak.color),
            );
        }
        let line = palette.primary.strong.color;
        let points: Vec<_> = self
            .samples
            .iter()
            .enumerate()
            .filter_map(|(ply, score)| {
                score.map(|score| (ply, Point::new(x(ply), y(score.chart_value(limit)))))
            })
            .collect();
        if let Some(((_, first), rest)) = points.split_first() {
            let path = Path::new(|builder| {
                builder.move_to(*first);
                for &(_, point) in rest {
                    builder.line_to(point);
                }
            });
            frame.stroke(&path, Stroke::default().with_color(line).with_width(2.0));
            for (ply, point) in points {
                let radius = if ply == self.selected { 4.5 } else { 2.5 };
                frame.fill(&Path::circle(point, radius), line);
            }
        }

        vec![frame.into_geometry()]
    }
}

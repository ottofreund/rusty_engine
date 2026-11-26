use iced::widget::image::Handle;

macro_rules! piece_handle {
    ($path:expr) => {{
        // RGBA handles are ready for the first board layout.
        let sprite =
            image::load_from_memory_with_format(include_bytes!($path), image::ImageFormat::Png)
                .expect(concat!("invalid embedded piece sprite: ", $path))
                .into_rgba8();
        Handle::from_rgba(sprite.width(), sprite.height(), sprite.into_raw())
    }};
}

macro_rules! make_handles {
    ($w:literal, $b:literal) => {
        [
            piece_handle!(concat!($w, "plt.png")),
            piece_handle!(concat!($w, "nlt.png")),
            piece_handle!(concat!($w, "blt.png")),
            piece_handle!(concat!($w, "rlt.png")),
            piece_handle!(concat!($w, "qlt.png")),
            piece_handle!(concat!($w, "klt.png")),
            piece_handle!(concat!($b, "pdt.png")),
            piece_handle!(concat!($b, "ndt.png")),
            piece_handle!(concat!($b, "bdt.png")),
            piece_handle!(concat!($b, "rdt.png")),
            piece_handle!(concat!($b, "qdt.png")),
            piece_handle!(concat!($b, "kdt.png")),
        ]
    };
}

pub struct ImageHandle {
    pub img_handles: [Handle; 12],
    pub target_ball_handle: Handle,
    pub target_circle_handle: Handle,
}

impl Default for ImageHandle {
    fn default() -> Self {
        let handles: [Handle; 12] = make_handles!(
            "../../assets/default_piece_set/white_pieces/",
            "../../assets/default_piece_set/black_pieces/"
        );
        let target_ball_handle: Handle =
            Handle::from_bytes(include_bytes!("../../assets/target_ball.png").as_slice());
        let target_circle_handle: Handle =
            Handle::from_bytes(include_bytes!("../../assets/target_circle.png").as_slice());

        return Self {
            img_handles: handles,
            target_ball_handle,
            target_circle_handle,
        };
    }
}

use iced::widget::image::Handle;

macro_rules! embedded_handle {
    ($path:expr) => {
        Handle::from_bytes(include_bytes!($path).as_slice())
    };
}

macro_rules! make_handles {
    ($w:literal, $b:literal) => {
        [
            embedded_handle!(concat!($w, "plt.png")),
            embedded_handle!(concat!($w, "nlt.png")),
            embedded_handle!(concat!($w, "blt.png")),
            embedded_handle!(concat!($w, "rlt.png")),
            embedded_handle!(concat!($w, "qlt.png")),
            embedded_handle!(concat!($w, "klt.png")),

            embedded_handle!(concat!($b, "pdt.png")),
            embedded_handle!(concat!($b, "ndt.png")),
            embedded_handle!(concat!($b, "bdt.png")),
            embedded_handle!(concat!($b, "rdt.png")),
            embedded_handle!(concat!($b, "qdt.png")),
            embedded_handle!(concat!($b, "kdt.png")),
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
        let target_ball_handle: Handle = embedded_handle!("../../assets/target_ball.png");
        let target_circle_handle: Handle = embedded_handle!("../../assets/target_circle.png");

        return Self {
            img_handles: handles,
            target_ball_handle,
            target_circle_handle,
        };
    }
}
use iced::widget::{Image, image::{self, Handle}};

pub struct ImageHandle {
    pub img_handles: [Handle ; 12],
    pub target_ball_handle: Handle,
    pub target_circle_handle: Handle
}

impl Default for ImageHandle {
    fn default() -> Self {
        
        let handles: [Handle ; 12] = std::array::from_fn(|i| Handle::from_path(PATHS[i]));
        let target_ball_handle: Handle = Handle::from_path("./assets/target_ball.png");
        let target_circle_handle: Handle = Handle::from_path("./assets/target_circle.png");
        return Self { img_handles: handles, target_ball_handle, target_circle_handle}
    }
}

macro_rules! make_paths {
($w:expr, $b:expr) => {
[
concat!($w, "plt.png"),
concat!($w, "nlt.png"),
concat!($w, "blt.png"),
concat!($w, "rlt.png"),
concat!($w, "qlt.png"),
concat!($w, "klt.png"),
concat!($b, "pdt.png"),
concat!($b, "ndt.png"),
concat!($b, "bdt.png"),
concat!($b, "rdt.png"),
concat!($b, "qdt.png"),
concat!($b, "kdt.png"),
]
};
}

// use with literal prefixes:
const PATHS: [&str; 12] = make_paths!(
"./assets/default_piece_set/white_pieces/",
"./assets/default_piece_set/black_pieces/"
);
use rusty_engine::{
    game::{game::Game, game_state::GameState},
    repr::{board::Board, position::Position},
    utils::fen_tool,
};

const KINGS_ONLY_FEN: &str = "4k3/8/8/8/8/8/8/4K3 w - - 0 1";
const NO_LEGAL_EP_FEN: &str = "4k3/8/8/3p4/8/8/8/4K3 w - - 0 1";
const NO_LEGAL_EP_WITH_EP_SQUARE_FEN: &str = "4k3/8/8/3p4/8/8/8/4K3 w - d6 0 1";
const LEGAL_EP_FEN: &str = "4k3/8/8/3pP3/8/8/8/4K3 w - - 0 1";
const LEGAL_EP_WITH_EP_SQUARE_FEN: &str = "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1";

#[test]
fn on_board_detects_natural_threefold_repetition() {
    let mut game = Game::default();

    for (idx, (from, to)) in [
        ("g1", "f3"),
        ("g8", "f6"),
        ("f3", "g1"),
        ("f6", "g8"),
        ("g1", "f3"),
        ("g8", "f6"),
        ("f3", "g1"),
        ("f6", "g8"),
    ]
    .into_iter()
    .enumerate()
    {
        play(&mut game, from, to);

        if idx == 3 {
            assert_in_progress(&game);
        }
    }

    assert_draw_by_repetition(&game);
}

#[test]
fn on_board_threefold_uses_board_state_instead_of_zobrist_hash() {
    let mut game = Game::default();
    import_position(&mut game, KINGS_ONLY_FEN);

    let mut first_repetition = board(&game, KINGS_ONLY_FEN);
    let mut second_repetition = first_repetition.clone();
    first_repetition.zhash ^= 0xBAD5_EED;
    second_repetition.zhash ^= 0xFACE_FEED;
    game.board_history = vec![first_repetition, second_repetition];

    play_king_cycle(&mut game);

    assert_draw_by_repetition(&game);
}

#[test]
fn on_board_threefold_ignores_ep_square_without_legal_ep_capture() {
    let mut game = Game::default();
    import_position(&mut game, NO_LEGAL_EP_FEN);

    game.board_history = vec![
        board(&game, NO_LEGAL_EP_WITH_EP_SQUARE_FEN),
        board(&game, NO_LEGAL_EP_FEN),
    ];

    play_king_cycle(&mut game);

    assert_draw_by_repetition(&game);
}

#[test]
fn on_board_threefold_distinguishes_legal_ep_capture_right() {
    let mut game = Game::default();
    import_position(&mut game, LEGAL_EP_FEN);

    game.board_history = vec![
        board(&game, LEGAL_EP_WITH_EP_SQUARE_FEN),
        board(&game, LEGAL_EP_FEN),
    ];

    play_king_cycle(&mut game);

    assert_in_progress(&game);
}

fn import_position(game: &mut Game, fen: &str) {
    let position =
        Position::position_with(fen, &game.move_gen, &game.zobrist).expect("valid FEN position");
    game.import_position(position);
}

fn board(game: &Game, fen: &str) -> Board {
    fen_tool::fen_to_board(fen.to_owned(), &game.move_gen, &game.zobrist).expect("valid FEN board")
}

fn play_king_cycle(game: &mut Game) {
    for (from, to) in [("e1", "f1"), ("e8", "f8"), ("f1", "e1"), ("f8", "e8")] {
        play(game, from, to);
    }
}

fn play(game: &mut Game, from: &str, to: &str) {
    game.try_make_move(square(from), square(to))
        .unwrap_or_else(|_| panic!("expected legal move {from}{to}"));
}

fn square(name: &str) -> u32 {
    let bytes = name.as_bytes();
    assert_eq!(bytes.len(), 2);
    (bytes[0] - b'a') as u32 + 8 * (bytes[1] - b'1') as u32
}

fn assert_draw_by_repetition(game: &Game) {
    assert!(
        matches!(&game.game_state, GameState::DrawByRepetition),
        "expected draw by repetition, got {}",
        game.game_state.to_string()
    );
}

fn assert_in_progress(game: &Game) {
    assert!(
        matches!(&game.game_state, GameState::InProgress),
        "expected game to remain in progress, got {}",
        game.game_state.to_string()
    );
}

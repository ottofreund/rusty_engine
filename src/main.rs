use iced::futures::executor::block_on;
use rusty_engine::{game::cpu_game, uci::command_listener::listen};

fn main() {
    //app::run_fr().unwrap();
    let cpu_game = cpu_game::CpuGame::default();
    block_on(listen(cpu_game));
}

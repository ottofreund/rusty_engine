use iced::futures::executor::block_on;
use rusty_engine::{game::cpu_game, repr::types::VERSION, uci::command_listener::listen, ui::app};

fn main() {
    let uci_mode: bool = true;
    if uci_mode {
        println!("Rusty Engine v{} by Otto Freund", VERSION);
        let cpu_game = cpu_game::CpuGame::default();
        block_on(listen(cpu_game));
    } else {
        app::run_fr().unwrap();
    }
}

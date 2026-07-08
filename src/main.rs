use rusty_engine::{game::cpu_game, uci::command_listener::listen, ui::*};

fn main() {
    //app::run_fr().unwrap();
    let cpu_game = cpu_game::CpuGame::default();
    listen(cpu_game);
}

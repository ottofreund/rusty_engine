use rusty_engine::{game::cpu_game, repr::types::VERSION, uci::command_listener::listen};

#[tokio::main]
async fn main() {
    println!("Rusty Engine v{} by Otto Freund", VERSION);
    let cpu_game = cpu_game::CpuGame::default();

    listen(cpu_game).await;
}

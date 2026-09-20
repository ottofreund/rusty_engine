use rusty_engine::{bench, game::cpu_game, repr::types::VERSION, uci::command_listener::listen};

#[tokio::main]
async fn main() {
    if std::env::args().nth(1).as_deref() == Some("bench") {
        let result = bench::run();
        println!("{} nodes {} nps", result.nodes, result.nps);
        return;
    }

    println!("Rusty Engine v{} by Otto Freund", VERSION);
    let cpu_game = cpu_game::CpuGame::default();

    listen(cpu_game).await;
}

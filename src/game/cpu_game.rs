use crate::{
    repr::{move_gen::MoveGen, position::Position},
    search::searcher::Searcher,
    utils::zobrist::Zobrist,
};

/// Game object that is optimized for CPU vs CPU games, no need for game state tracking
/// Effectively represents a CPU player
pub struct CpuGame {
    pub position: Position,
    pub searcher: Searcher,
    pub move_gen: MoveGen,
    pub zobrist: Zobrist,
}

impl CpuGame {
    //pub fn import_time_control()
}

impl Default for CpuGame {
    fn default() -> Self {
        let move_gen = MoveGen::init();
        let zobrist = Zobrist::default();
        let position = Position::default(&move_gen, &zobrist);
        let searcher = Searcher::from(&position);
        Self {
            position,
            searcher,
            move_gen,
            zobrist,
        }
    }
}

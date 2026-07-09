use crate::{
    repr::{_move::{self, from_string, promotion_matches}, move_gen::MoveGen, position::Position}, search::searcher::Searcher, utils::zobrist::Zobrist,
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

    /// With **moves** played from **base_position**
    /// **moves** are UCI formatted, can be empty
    pub fn import_position(&mut self, mut base_position: Position, moves: Vec<String>) -> Result<(), String> {
        let mut board_hash_history: Vec<u64> = vec![base_position.board.zhash];
        let mut only_sync: bool = false; // if new is cur + some move, can sync instead of importing, which allows to optimize next search
        let mut last_move: Option<u32> = None;
        for (i, m) in moves.iter().enumerate() {
            let (from, to, promotion): (u32, u32, Option<u32>) =
                match from_string(&m) {
                    Ok(fromtoprom) => fromtoprom,
                    Err(err) => {
                        return Err(format!("Invalid move {}: {}", m, err));
                    }
                };
            let mov: u32 =
                match base_position.legal_moves().iter().copied().find(|mov| {
                    _move::get_init(*mov) == from
                        && _move::get_target(*mov) == to
                        && promotion_matches(*mov, promotion)
                }) {
                    Some(mov) => mov,
                    None => {
                        return Err(format!("Illegal move: {}", m));
                    }
                };
            
            if i == moves.len() - 1 && self.position.board == base_position.board {
                only_sync = true;
                last_move = Some(mov);
            }

            if _move::is_unrepeatable(mov) {
                board_hash_history.clear();
            }
            base_position.make_move(mov, false, false, &self.move_gen, &self.zobrist);
            board_hash_history.push(base_position.board.zhash);
        }

        if only_sync {
            self.searcher.sync_new_move(&base_position, last_move);
        } else {
            self.searcher.import_position(&base_position, Some(board_hash_history));
        }
        self.position = base_position;
        Ok(())
    }

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

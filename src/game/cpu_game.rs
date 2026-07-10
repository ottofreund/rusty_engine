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
    pub fn import_position(&mut self, base_pos_fen: &str, moves: Vec<String>) -> Result<(), String> {
        let mut new_pos =
            match Position::from(base_pos_fen, &self.move_gen, &self.zobrist) {
                Ok(position) => position,
                Err(err) => {
                    return Err(format!("Invalid FEN: {}", err));
                }
            };
        let mut board_hash_history: Vec<u64> = vec![new_pos.board.zhash];

        for (i, m) in moves.iter().enumerate() {
            let (from, to, promotion): (u32, u32, Option<u32>) =
                match from_string(&m) {
                    Ok(fromtoprom) => fromtoprom,
                    Err(err) => {
                        return Err(format!("Invalid move {}: {}", m, err));
                    }
                };
            let mov: u32 =
                match new_pos.legal_moves().iter().copied().find(|mov| {
                    _move::get_init(*mov) == from
                        && _move::get_target(*mov) == to
                        && promotion_matches(*mov, promotion)
                }) {
                    Some(mov) => mov,
                    None => {
                        return Err(format!("Illegal move: {}", m));
                    }
                };
            if _move::is_unrepeatable(mov) {
                board_hash_history.clear();
            }
            new_pos.make_move(mov, false, false, &self.move_gen, &self.zobrist);
            board_hash_history.push(new_pos.board.zhash);
        }

        self.searcher.import_position(&new_pos, Some(board_hash_history));
        self.position = new_pos;
        Ok(())
    }

    pub fn sync_new_move(&mut self, mov: &str) -> Result<(), String> {
        let mov: u32 = match from_string(mov) {
            Ok((from, to, promotion)) => match self.position.legal_moves().iter().copied().find(|mov| {
                _move::get_init(*mov) == from
                    && _move::get_target(*mov) == to
                    && promotion_matches(*mov, promotion)
            }) {
                Some(mov) => mov,
                None => {
                    return Err(format!("Illegal move: {}", mov));
                }
            },
            Err(err) => {
                return Err(format!("Invalid move {}: {}", mov, err));
            }
        };
        self.position.make_move(mov, false, false, &self.move_gen, &self.zobrist);
        self.searcher.sync_new_move(&self.position, Some(mov));
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

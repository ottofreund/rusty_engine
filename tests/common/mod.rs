#![allow(dead_code)]

use rusty_engine::{
    repr::{board::Board, move_gen::MoveGen, position::Position},
    utils::{
        fen_tool::{self, DEFAULT_FEN},
        zobrist::Zobrist,
    },
};

pub struct TestEngine {
    pub move_gen: MoveGen,
    pub zobrist: Zobrist,
}

impl TestEngine {
    pub fn new() -> Self {
        Self {
            move_gen: MoveGen::init(),
            zobrist: Zobrist::default(),
        }
    }

    pub fn position(&self, fen: &str) -> Position {
        Position::position_with(fen, &self.move_gen, &self.zobrist).expect("valid FEN")
    }

    pub fn board(&self, fen: &str) -> Board {
        fen_tool::fen_to_board(fen.to_owned(), &self.move_gen, &self.zobrist).expect("valid FEN")
    }

    pub fn default_board(&self) -> Board {
        Board::default_board(&self.move_gen, &self.zobrist)
    }

    pub fn recomputed_hash(&self, board: &Board) -> u64 {
        self.zobrist.init_hash(board)
    }

    pub fn make_search_move(&self, pos: &mut Position, mov: u32) {
        pos.make_move(mov, true, false, &self.move_gen, &self.zobrist);
    }

    pub fn unmake_move(&self, pos: &mut Position, mov: u32) {
        pos.unmake_move(mov, &self.zobrist);
    }

    pub fn perft(&self, target_depth: usize, pos: &mut Position) -> u32 {
        go_perft(target_depth, pos, &self.move_gen, &self.zobrist)
    }
}

impl Default for TestEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy)]
pub struct PerftCase {
    pub name: &'static str,
    pub fen: &'static str,
    pub depth: usize,
    pub expected: u32,
}

pub const PERFT_CASES: [PerftCase; 6] = [
    PerftCase {
        name: "default position",
        fen: DEFAULT_FEN,
        depth: 6,
        expected: 119_060_324,
    },
    PerftCase {
        name: "kiwipete",
        fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ",
        depth: 5,
        expected: 193_690_690,
    },
    PerftCase {
        name: "edge case 3",
        fen: "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        depth: 7,
        expected: 178_633_661,
    },
    PerftCase {
        name: "edge case 4",
        fen: "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        depth: 5,
        expected: 15_833_292,
    },
    PerftCase {
        name: "edge case 5",
        fen: "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        depth: 5,
        expected: 89_941_194,
    },
    PerftCase {
        name: "edge case 6",
        fen: "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
        depth: 5,
        expected: 164_075_551,
    },
];

pub fn go_perft(
    target_depth: usize,
    pos: &mut Position,
    move_gen: &MoveGen,
    zobrist: &Zobrist,
) -> u32 {
    assert!(target_depth > 1);

    fn inner(d_left: usize, pos: &mut Position, move_gen: &MoveGen, zobrist: &Zobrist) -> usize {
        if d_left == 1 {
            return pos.legal_search_moves().len();
        }

        let mut from_here = 0;
        let (s, e) = pos.search_move_bounds();
        for i in s..e {
            let mov = pos.move_arr[i];
            pos.make_move(mov, true, true, move_gen, zobrist);
            from_here += inner(d_left - 1, pos, move_gen, zobrist);
            pos.unmake_move(mov, zobrist);
        }

        from_here
    }

    inner(target_depth, pos, move_gen, zobrist) as u32
}

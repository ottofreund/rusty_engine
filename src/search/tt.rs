//* Stockfish inspired implementation *//

use crate::{
    repr::{
        _move::{self, NULL_MOVE},
        move_gen::MoveGen,
        position::Position,
    },
    search::eval::MATE_BOUND,
    utils::zobrist::Zobrist,
};

const CLUSTER_SIZE: usize = 4;
const REPLACE_V_AGE_COEFFICIENT: u16 = 4;
pub const DEFAULT_TT_SIZE: usize = 16 * 1024 * 1024; // == 16 MiB

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TTEntryType {
    Exact = 0,
    LowerBound = 1,
    UpperBound = 2,
}

const NULL_SENTINEL: u8 = u8::MAX;
const NULL_ENTRY: TTEntry = TTEntry {
    key: 0,
    best_move: 0,
    depth_and_bound_type: NULL_SENTINEL,
    score: 0,
    generation: 0,
};
const NULL_ENTRY_VALUE: i16 = -10_000;

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub key: u64,
    pub best_move: u32,
    pub depth_and_bound_type: u8, //6 LSB are depth, 2 MSB are bound type
    pub score: i16,
    pub generation: u8,
}
impl TTEntry {

    pub fn new_packed(key: u64, best_move: u32, depth: u8, bound_type: TTEntryType, score: i16, generation: u8) -> Self {
        let depth_and_bound_type: u8 = (depth & 0b111111) | ((bound_type as u8) << 6);
        return Self {
            key,
            best_move,
            depth_and_bound_type,
            score,
            generation,
        }
    }

    #[inline]
    pub fn is_occupied(&self) -> bool {
        self.depth_and_bound_type != NULL_SENTINEL
    }

    #[inline]
    pub fn depth(&self) -> u8 {
        self.depth_and_bound_type & 0b111111
    }

    #[inline]
    pub fn bound_type(&self) -> TTEntryType {
        match (self.depth_and_bound_type >> 6) & 0b11 {
            0 => TTEntryType::Exact,
            1 => TTEntryType::LowerBound,
            2 => TTEntryType::UpperBound,
            _ => unreachable!(),
        }
    }

}


#[repr(align(64))]
#[derive(Clone, Copy)]
pub struct TTCluster {
    pub entries: [TTEntry; CLUSTER_SIZE],
}

//compile time checks s.t. fits into cache line and aligns nicely
const _: () = assert!(size_of::<TTEntry>() == 16);
const _: () = assert!(size_of::<TTCluster>() == 64);
const _: () = assert!(align_of::<TTCluster>() == 64);

pub struct TranspositionTable {
    pub clusters: Box<[TTCluster]>,
    pub nof_clusters: usize,
    pub generation: u8,
}

impl TranspositionTable {

    pub fn resize(&mut self, size_mb: u32) {
        let new_nof_clusters = (size_mb as usize * 1024 * 1024) / std::mem::size_of::<TTCluster>();
        if new_nof_clusters != self.nof_clusters {
            self.clusters = vec![TTCluster {
                entries: [NULL_ENTRY ; CLUSTER_SIZE],
            }; new_nof_clusters]
            .into_boxed_slice();
            self.nof_clusters = new_nof_clusters;
        }
    }

    pub fn clear(&mut self) {
        for cluster in self.clusters.iter_mut() {
            for entry in cluster.entries.iter_mut() {
                *entry = NULL_ENTRY;
            }
        }
        self.generation = 0;
    }

    /// If hit, returns (true, entry) else returns (false, entry_to_replace)
    pub fn probe(&self, key: u64) -> Option<TTEntry> {
        let cluster_index: usize = self.get_cluster_idx(key);
        let cluster: &TTCluster = &self.clusters[cluster_index];

        for i in 0..CLUSTER_SIZE {
            if cluster.entries[i].key == key && cluster.entries[i].is_occupied() {
                return Some(cluster.entries[i]);
            }
        }
        //no hit
        return None;
    }

    pub fn store(&mut self, tte: TTEntry) {
        let cluster_index: usize = self.get_cluster_idx(tte.key);
        let cluster: &mut TTCluster = &mut self.clusters[cluster_index];

        //find least valuable / existing same key entry to replace 
        let mut replace_i: usize = 0;
        let mut replace_entry_v = i16::MAX;
        for i in 0..CLUSTER_SIZE {
            if cluster.entries[i].key == tte.key && cluster.entries[i].is_occupied() {
                let existing_d: u8 = cluster.entries[i].depth();
                let tte_depth: u8 = tte.depth();
                if tte_depth > existing_d || (tte_depth == existing_d && tte.bound_type() == TTEntryType::Exact) { //replace existing same key entry if geq depth
                    cluster.entries[i] = tte;
                }
                return;
            }
            let cur_entry_v = Self::entry_value(self.generation, &cluster.entries[i]);
            if replace_entry_v > cur_entry_v {
                replace_i = i;
                replace_entry_v = cur_entry_v;
            }
        }
        cluster.entries[replace_i] = tte;
    }

    #[inline]
    fn entry_value(cur_gen: u8, entry: &TTEntry) -> i16 {
        if !entry.is_occupied() {
            return NULL_ENTRY_VALUE;
        } else {
            let age: i16 = cur_gen.wrapping_sub(entry.generation) as i16;
            return entry.depth() as i16 - REPLACE_V_AGE_COEFFICIENT as i16 * age;
        }
        
    }

    #[inline]
    fn get_cluster_idx(&self, key: u64) -> usize {
        return (((key as u128) * (self.nof_clusters as u128)) >> 64) as usize;
    }

    // Important that mate scores are stored relative to node stored from and not search root
    pub fn score_to_tt(score: i16, ply: i16) -> i16 {
        if score >= MATE_BOUND {
            return score + ply;
        } else if score <= -MATE_BOUND {
            return score - ply;
        } else {
            return score;
        }
    }

    pub fn score_from_tt(score: i16, ply: i16) -> i16 {
        if score >= MATE_BOUND {
            return score - ply;
        } else if score <= -MATE_BOUND {
            return score + ply;
        } else {
            return score;
        }
    }

    /// Replays and validates the searched PV prefix from `root`, then extends
    /// its first gap through exact, sufficiently deep TT entries.
    /// Returns the number of moves retained and clears the unused output tail.
    pub fn reconstruct_pv(
        &self,
        root: &Position,
        ply_pv_slice: &mut [u32],
        board_hash_history: &[u64],
        move_gen: &MoveGen,
        zobrist: &Zobrist,
    ) -> usize {
        let mut replay = root.clone();
        let mut replay_history = Vec::with_capacity(
            board_hash_history.len() + ply_pv_slice.len() + 1,
        );
        replay_history.extend_from_slice(board_hash_history);
        if replay_history.last().copied() != Some(root.zhash) {
            replay_history.push(root.zhash);
        }

        let mut added: usize = 0;
        let mut following_searched_prefix = true;
        while added < ply_pv_slice.len() {
            let is_threefold = replay_history
                .iter()
                .rev()
                .step_by(2)
                .filter(|hash| **hash == replay.zhash)
                .take(3)
                .count()
                >= 3;

            if is_threefold
                || replay.board.is_fifty_move_draw()
                || replay.legal_search_moves().is_empty()
            {
                break;
            }

            let searched_move = ply_pv_slice[added];
            let mov = if following_searched_prefix && searched_move != NULL_MOVE {
                searched_move
            } else {
                following_searched_prefix = false;
                if replay.board.half_move_clock >= 96 {
                    break;
                }

                let Some(entry) = self.probe(replay.zhash) else {
                    break;
                };
                let remaining_depth = ply_pv_slice.len() - added;
                if entry.bound_type() != TTEntryType::Exact
                    || (entry.depth() as usize) < remaining_depth
                    || entry.best_move == NULL_MOVE
                {
                    break;
                }
                entry.best_move
            };

            let is_legal = replay.legal_search_moves().contains(&mov);
            if !is_legal {
                break;
            }

            ply_pv_slice[added] = mov;
            added += 1;
            replay.make_move(mov, false, false, false, move_gen, zobrist);
            if _move::is_unrepeatable(mov) {
                replay_history.clear();
            }
            replay_history.push(replay.zhash);
        }

        ply_pv_slice[added..].fill(NULL_MOVE);
        added
    }

}

impl Default for TranspositionTable {
    fn default() -> Self {
        let nof_clusters: usize = DEFAULT_TT_SIZE / std::mem::size_of::<TTCluster>();
        let clusters: Box<[TTCluster]> = vec![TTCluster {
            entries: [NULL_ENTRY ; CLUSTER_SIZE],
        }; nof_clusters]
        .into_boxed_slice();

        return Self {
            clusters,
            nof_clusters,
            generation: 0,
        }
    }
}

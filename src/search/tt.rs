//* Stockfish inspired implementation *//

const CLUSTER_SIZE: usize = 3;
const REPLACE_V_AGE_COEFFICIENT: u16 = 4;
pub const DEFAULT_TT_SIZE: u32 = 16 * 1024 * 1024; // 16 MiB

#[derive(Clone, Copy)]
pub enum TTEntryType {
    Exact = 0,
    LowerBound = 1,
    UpperBound = 2,
}

const NULL_DEPTH_SENTINEL: u8 = 0;
const NULL_ENTRY: TTEntry = TTEntry {
    key: 0,
    best_move: 0,
    depth: NULL_DEPTH_SENTINEL,
    score: 0,
    _type: TTEntryType::Exact,
    generation: 0,
};

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub key: u64,
    pub best_move: u32,
    pub depth: u8,
    pub score: i32,
    pub _type: TTEntryType,
    pub generation: u8,
}
impl TTEntry {

    #[inline]
    pub fn is_occupied(&self) -> bool {
        self.depth != NULL_DEPTH_SENTINEL
    }

}


#[derive(Clone, Copy)]
pub struct TTCluster {
    pub entries: [TTEntry; CLUSTER_SIZE],
}

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

    /// If hit, returns (true, entry) else returns (false, entry_to_replace)
    pub fn probe(&mut self, key: u64) -> (bool, &mut TTEntry) {
        let cluster_index: usize = self.get_cluster_idx(key);
        let cluster: &mut TTCluster = &mut self.clusters[cluster_index];

        for i in 0..CLUSTER_SIZE {
            if cluster.entries[i].key == key && cluster.entries[i].is_occupied() {
                return (true, &mut cluster.entries[i]);
            }
        }
        
        //no hit, find least valuable entry to replace
        let mut replace_i: usize = 0;
        let mut replace_v = Self::replacement_value(&cluster.entries[0]);
        for i in 1..CLUSTER_SIZE {
            let cur_replace_v = Self::replacement_value(&cluster.entries[i]);
            if replace_v > cur_replace_v {
                replace_i = i;
                replace_v = cur_replace_v;
            }
        }
        return (false, &mut cluster.entries[replace_i]);
    }

    #[inline]
    fn replacement_value(entry: &TTEntry) -> i16 {
        return entry.depth as i16 - REPLACE_V_AGE_COEFFICIENT as i16 * entry.generation as i16 - if entry.is_occupied() { 0 } else { 1000 };
    }

    #[inline]
    fn get_cluster_idx(&self, key: u64) -> usize {
        return (((key as u128) * (self.nof_clusters as u128)) >> 64) as usize;
    }

}

impl Default for TranspositionTable {
    fn default() -> Self {
        let nof_clusters: usize = (DEFAULT_TT_SIZE as usize * 1024 * 1024) / std::mem::size_of::<TTCluster>();
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
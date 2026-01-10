use std::collections::HashSet;
use rand::prelude::*;
use crate::repr::bitboard;
use crate::repr::move_gen::{self, naive_rook_sliding, naive_bishop_sliding};
///MagicBitboard object is initialized on startup and holds all relevant data and methods related to initializing magic bitboards and using them.
pub struct MagicBitboard {
    pub rook_slide_bbs: Vec<Vec<u64>>, //'legal' slides magically indexed by **relevant** blocker masks
    pub bishop_slide_bbs: Vec<Vec<u64>>, //vec because multi-megabyte array too large for stack
    rook_magics: [u64 ; 64], //rook magic multipliers for each square
    bishop_magics: [u64 ; 64] //for bishop
}

impl MagicBitboard {
    ///Initializes magic bitboards by finding a working magic number (multiplier and shift amount) for each square. Returns Self which provides API to easily fetch 'legal' slide bbs magically indexed.
    /// new_magics flag can be toggled if want to compute new magics (startup takes a few more seconds) instead of using the precomputed memoized ones.
    pub fn init_magic(empty_board_attack_bbs: &[[u64 ; 64] ; 12], rook_empty_board_attack_bbs_no_edges: &[u64 ; 64], bishop_empty_board_attack_bbs_no_edges: &[u64 ; 64], new_magics: bool) -> Self {
        let mut rook_magics: [u64 ; 64] = ROOK_MAGICS_MEMOIZED;
        let mut bishop_magics: [u64 ; 64] = BISHOP_MAGICS_MEMOIZED;
        let mut rook_slide_bbs: Vec<Vec<u64>> = Vec::new(); //these are filled "on the fly" square by square so that each square reserves as little space as it actually needs (squares have varying need of nof bits == inner vec len)
        let mut bishop_slide_bbs: Vec<Vec<u64>> = Vec::new();

        let mut rng: ThreadRng = rand::rng();
        for sqr in 0..64 {  
            let rook_empty_board_attack_bb: u64 = empty_board_attack_bbs[3][sqr];
            let rook_empty_board_attack_bb_no_edges: u64 = rook_empty_board_attack_bbs_no_edges[sqr];
            let bishop_empty_board_attack_bb: u64 = empty_board_attack_bbs[2][sqr as usize];
            let bishop_empty_board_attack_bb_no_edges: u64 = bishop_empty_board_attack_bbs_no_edges[sqr];
            //we don't want edges here with the following, lookups are done with relevant blocker bitboards
            let all_rook_block_masks: Vec<u64> = move_gen::generate_all_blocker_masks(rook_empty_board_attack_bb, Some(rook_empty_board_attack_bb_no_edges));
            let all_bishop_block_masks: Vec<u64> = move_gen::generate_all_blocker_masks(bishop_empty_board_attack_bb, Some(bishop_empty_board_attack_bb_no_edges));
            //look for magics if want new ones
            if new_magics {
                //for rook
                find_working(sqr as u32, true, &mut rook_magics, &all_rook_block_masks,  &mut rng);
                //for bishop
                find_working(sqr as u32, false,  &mut bishop_magics, &all_bishop_block_masks, &mut rng);
            }          
            //now we can build the inner magic indexed vectors for this sqr with legal slides and push to outer vec
            let mut rook_lookup_vec: Vec<u64> = vec![0u64 ; 1 << ROOK_BITS[sqr]];
            for block_mask in all_rook_block_masks { //for rook
                let magic_idx: usize = ((block_mask.wrapping_mul(rook_magics[sqr])) >> ROOK_SHIFTS[sqr]) as usize;
                rook_lookup_vec[magic_idx] = naive_rook_sliding(sqr as u32, block_mask, true);
            }
            let mut bishop_lookup_vec: Vec<u64> = vec![0u64 ; 1 << BISHOP_BITS[sqr]];
            for block_mask in all_bishop_block_masks { //for bishop
                let magic_idx: usize = ((block_mask.wrapping_mul(bishop_magics[sqr])) >> BISHOP_SHIFTS[sqr]) as usize;
                bishop_lookup_vec[magic_idx] = naive_bishop_sliding(sqr as u32, block_mask, true);
            }
            rook_slide_bbs.push(rook_lookup_vec); //add this sqr to outer lookup vec
            bishop_slide_bbs.push(bishop_lookup_vec);
        }
        //now magics computed and 'legal' slide bbs filled magically indexed
        return Self { rook_magics, bishop_magics, rook_slide_bbs, bishop_slide_bbs }
    }
    ///For piece at **sqr** give blocker bitboard (ONLY RELEVANT BLOCKERS) and get corresponding magic idx with which to lookup from arr.
    ///Blocker bitboard has to be masked with no_edge bbs to contain only relevant blockers
    pub fn get_magic_idx(&self, sqr: usize, rel_blockers: u64, rook: bool) -> usize {
        if rook {
            return ((rel_blockers.wrapping_mul(self.rook_magics[sqr])) >> ROOK_SHIFTS[sqr]) as usize;
        } else {
            return ((rel_blockers.wrapping_mul(self.bishop_magics[sqr])) >> BISHOP_SHIFTS[sqr]) as usize;
        }
    }

}

///Find working magic number for **sqr** and update result to collections.
/// **all_block_masks** are RELEVANT blocker masks
fn find_working(sqr: u32, rook: bool, magic_arr: &mut [u64 ; 64], all_block_masks: &Vec<u64>, rng: &mut ThreadRng) {
    let piece_idx: usize;
    let bits_needed: u32;
    if rook {piece_idx = 3; bits_needed = ROOK_BITS[sqr as usize];} else {piece_idx = 2; bits_needed = BISHOP_BITS[sqr as usize];}
    let shift_amount: u32 = 64 - bits_needed;
    //now try different random magic numbers until find one that works for all masks
    let mut iterations = 0;
    let used_capacity: usize = 1 << bits_needed; //for **used** vec
    loop {
        iterations += 1;
        let mut collided: bool = false;
        let mut used: Vec<bool> = vec![false; used_capacity]; //used indices, if collision magic number doesn't work
        let magic: u64 = gen_random_magic(rng);
        for block_mask in all_block_masks {
            let magic_idx: usize = (((*block_mask).wrapping_mul(magic)) >> shift_amount) as usize;
            if used[magic_idx] { //collision, this magic doesn't work
                collided = true;
                break;
            }
            //no collision, mark this as used
            used[magic_idx] = true;
        }
        if !collided { //done, found working magic
            magic_arr[sqr as usize] = magic; //insert into magic arr
            println!("found magic with {} iterations", iterations);
            break;
        }
        if iterations > 1_000_000 {
            panic!("COULDNT FIND MAGIC, TIMED OUT");
        }
    }
    return;
}

///Get random magic number (the multiplier)
fn gen_random_magic(rng: &mut ThreadRng) -> u64 {
    return rng.next_u64() & rng.next_u64() & rng.next_u64();
}

///We precompute the magics here so startup is a few seconds faster
const ROOK_MAGICS_MEMOIZED: [u64 ; 64] = [36028866812190857, 918751921001480195, 180152849915977728, 36033745089742848, 36037594497941632, 9295432002603846656, 288232643903358017, 144115471560507428, 2307109648496377856, 1225542194631573760, 578994234328089344, 2414773893920760576, 10988923845510957056, 563109135777800, 19421781983166660, 306385530411553024, 141287298695722, 634143599763464, 477524498657054784, 324832019534078209, 19475649597015296, 4981122475183057408, 9228016925085270528, 18720285311192201, 144748577640579360, 2130202899698425857, 35186520623232, 2490526061776470032, 72061994240313344, 1173750867632328724, 13835111123898598913, 11529637269273067776, 9372167021304744576, 288799374526391557, 3603477849257808192, 18156544822611968, 5044172595289792768, 1174314171928421888, 144132788918980968, 6926817704036794434, 35459258417152, 18089577885614083, 1152992560562634773, 3459927797391818816, 144823273698394240, 9570166522642448, 581001805522272257, 290483280856350740, 1839491547406848, 322159092172032, 4647734608805249536, 2307109680969678976, 6599487923712, 281500813625600, 144187760273556480, 16343707789083136, 9147945618382913, 3179964649974019105, 11000359024214540418, 4785386543906981, 579275519385739266, 281483701125185, 216181652061160580, 1153202982033039681];

const BISHOP_MAGICS_MEMOIZED: [u64 ; 64] = [4543491443261712, 1172062919782072705, 24806807717281921, 1180093748382557265, 4756932470833942691, 586189300039750304, 23223889452598016, 720857694596186176, 1155191034062586498, 607485946888770, 9241395268031645952, 2258536570814496, 3477962004571103232, 4611704727852550144, 144188857780291616, 2310364477113241634, 581546013446245393, 40570914206714880, 9086510163034496, 579849481556598784, 1125994482180096, 281483571495432, 4684869651992219908, 227467004544746568, 58617163967374338, 1131019776370688, 9578971088289888, 1127004801861760, 72700259725221888, 73506751185436944, 36321267196363040, 883272119052474368, 613655049008066585, 6954721213199417664, 3459333107428362241, 4647787389656760448, 653094516018512000, 18369549374128193, 185211093021427720, 2310487967019696640, 2779295199698886784, 10380939545293168640, 9026994776838144, 578712827297009728, 76574426929308160, 2314871103517901064, 2259501260473344, 581106231889477792, 290772451599069187, 13844138105908498464, 1225102385815748608, 17798890258465, 297237644763594756, 2323927982660423828, 2343033183173640210, 22535663351005217, 761673511916028992, 9441801334790816321, 1155173374314955778, 2449963832292976640, 577305178307831296, 5188147389476702274, 1152947927265051169, 1155208523185944609];

///How many bits are used to index sqr at idx in magic computation
const ROOK_BITS: [u32 ; 64] = [
  12, 11, 11, 11, 11, 11, 11, 12,
  11, 10, 10, 10, 10, 10, 10, 11,
  11, 10, 10, 10, 10, 10, 10, 11,
  11, 10, 10, 10, 10, 10, 10, 11,
  11, 10, 10, 10, 10, 10, 10, 11,
  11, 10, 10, 10, 10, 10, 10, 11,
  11, 10, 10, 10, 10, 10, 10, 11,
  12, 11, 11, 11, 11, 11, 11, 12
];

const ROOK_SHIFTS: [u32 ; 64] = [
  52, 53, 53, 53, 53, 53, 53, 52,
  53, 54, 54, 54, 54, 54, 54, 53,
  53, 54, 54, 54, 54, 54, 54, 53,
  53, 54, 54, 54, 54, 54, 54, 53,
  53, 54, 54, 54, 54, 54, 54, 53,
  53, 54, 54, 54, 54, 54, 54, 53,
  53, 54, 54, 54, 54, 54, 54, 53,
  52, 53, 53, 53, 53, 53, 53, 52
];

const BISHOP_BITS: [u32 ; 64] = [
  6, 5, 5, 5, 5, 5, 5, 6,
  5, 5, 5, 5, 5, 5, 5, 5,
  5, 5, 7, 7, 7, 7, 5, 5,
  5, 5, 7, 9, 9, 7, 5, 5,
  5, 5, 7, 9, 9, 7, 5, 5,
  5, 5, 7, 7, 7, 7, 5, 5,
  5, 5, 5, 5, 5, 5, 5, 5,
  6, 5, 5, 5, 5, 5, 5, 6
];

const BISHOP_SHIFTS: [u32 ; 64] = [
  58, 59, 59, 59, 59, 59, 59, 58,
  59, 59, 59, 59, 59, 59, 59, 59,
  59, 59, 57, 57, 57, 57, 59, 59,
  59, 59, 57, 55, 55, 57, 59, 59,
  59, 59, 57, 55, 55, 57, 59, 59,
  59, 59, 57, 57, 57, 57, 59, 59,
  59, 59, 59, 59, 59, 59, 59, 59,
  58, 59, 59, 59, 59, 59, 59, 58
];


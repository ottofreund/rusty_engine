use crate::{
    repr::{_move::*, board::Board, move_gen::MoveGen},
    search::{eval::PIECE_MATERIAL_VALUE, search_data::SearchData},
};

const PROMOTION_SCORE: i32 = 1_000;
const EATING_MULTIPLIER: i32 = 7;
const NON_CAPTURE_BONUS: i32 = 10_000;
const GOOD_CAPTURE_BONUS: i32 = 100_000;

pub(super) struct MovePicker {
    priority: [u32; 2],
}

impl MovePicker {
    pub(super) fn new(primary: u32, secondary: u32) -> Self {
        Self {
            priority: [primary, secondary],
        }
    }

    pub(super) fn next(
        &mut self,
        moves: &mut [u32],
        move_gen: &MoveGen,
        data: &mut SearchData,
        board: &Board,
    ) -> u32 {
        for priority in &mut self.priority {
            let wanted = std::mem::replace(priority, NULL_MOVE);
            if wanted != NULL_MOVE {
                if let Some(index) = moves.iter().position(|&mov| move_eq(mov, wanted)) {
                    moves.swap(0, index);
                    return moves[0];
                }
            }
        }

        let mut best = 0;
        let mut best_score = i32::MIN;
        for (index, mov) in moves.iter_mut().enumerate() {
            let score = if let Some(victim) = eaten_piece(*mov) {
                if !is_positive_see(*mov) && !is_negative_see(*mov) {
                    *mov = if data.legacy_see.see_positive(*mov, board, move_gen) {
                        with_positive_see(*mov)
                    } else {
                        with_negative_see(*mov)
                    };
                }
                EATING_MULTIPLIER * i32::from(PIECE_MATERIAL_VALUE[victim as usize])
                    + if is_positive_see(*mov) {
                        GOOD_CAPTURE_BONUS
                    } else {
                        0
                    }
            } else if best_score >= GOOD_CAPTURE_BONUS {
                continue;
            } else if is_promotion(*mov) {
                NON_CAPTURE_BONUS
            } else {
                NON_CAPTURE_BONUS + data.get_history_entry(board.turn, *mov)
            };
            let score = score
                + if is_promotion(*mov) {
                    PROMOTION_SCORE + get_promotion_piece(*mov) as i32
                } else {
                    0
                };
            if score > best_score {
                best = index;
                best_score = score;
            }
        }
        moves.swap(0, best);
        moves[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{repr::position::Position, utils::zobrist::Zobrist};

    #[test]
    fn preferred_moves_then_good_captures_quiets_and_losing_captures() {
        let move_gen = MoveGen::init();
        let pos = Position::from(
            "4k3/8/5p2/4p3/3Q4/8/p7/R3K3 w - - 0 1",
            &move_gen,
            &Zobrist::default(),
        )
        .unwrap();
        let mut data = SearchData::new(&pos);
        let find = |name| {
            *pos.legal_moves()
                .iter()
                .find(|&&mov| to_string(mov, true) == name)
                .unwrap()
        };
        let good = find("a1a2");
        let bad = find("d4e5");
        let quiet = find("e1f1");
        let other_quiet = find("e1d1");

        data.update_cutoff_history(pos.board.turn, quiet, &[other_quiet], 4);
        data.age_history();
        assert_eq!(data.get_history_entry(pos.board.turn, quiet), 21);
        assert_eq!(data.get_history_entry(pos.board.turn, other_quiet), -21);

        let mut moves = [bad, other_quiet, quiet, good];
        let mut picker = MovePicker::new(NULL_MOVE, NULL_MOVE);
        let first = picker.next(&mut moves, &move_gen, &mut data, &pos.board);
        assert!(move_eq(first, good));
        assert!(is_positive_see(first));
        assert_eq!(
            picker.next(&mut moves[1..], &move_gen, &mut data, &pos.board),
            quiet
        );
        assert!(is_negative_see(moves[3]));

        let mut moves = [good, quiet, bad];
        let mut picker = MovePicker::new(bad, good);
        assert_eq!(
            picker.next(&mut moves, &move_gen, &mut data, &pos.board),
            bad
        );
        assert_eq!(
            picker.next(&mut moves[1..], &move_gen, &mut data, &pos.board),
            good
        );
    }
}

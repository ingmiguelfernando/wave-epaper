//! Dice: a shake-to-roll game whose engine holds no drawing.
//!
//! The owner rolls one to three six-sided dice. `request_roll` fixes the
//! final faces at once (so a test can see them), then `FRAMES` calls to
//! `advance` play a tumbling animation and land on those faces. The view
//! layer draws; this module only decides what is shown.

/// Most dice the game rolls at once.
pub const MAX_DICE: usize = 3;
/// Dice the game starts with.
pub const DEFAULT_DICE: usize = 2;
/// Faces on every die. Six until the mockup says otherwise.
pub const SIDES: u8 = 6;
/// Animation frames per roll, including the landing frame.
pub const FRAMES: u8 = 3;
/// Totals kept in the history, newest first.
pub const HISTORY_LEN: usize = 6;
/// A full panel refresh is due after this many completed rolls.
pub const FULL_REFRESH_EVERY: u32 = 10;
/// xorshift keeps zero fixed, so a zero seed starts from this constant.
const ZERO_SEED: u32 = 0x9E37_79B9;

/// A roll in flight, or settled on its faces.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DicePhase {
    Ready,
    Rolling { frame: u8 },
}

/// One dice game: the faces on the table, the recent totals and the roll
/// animation. Every seeded game rolls the same faces, as `race.rs` deals
/// the same road.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiceGame {
    dice: usize,
    /// Count chosen with `set_dice`, applied when the phase is `Ready`.
    pending_dice: usize,
    /// Faces currently shown: the last result in `Ready`, tumbling while
    /// rolling.
    faces: [u8; MAX_DICE],
    /// Faces the in-flight roll will land on; ignored in `Ready`.
    final_faces: [u8; MAX_DICE],
    /// Tumbling faces of the previous frame, so two frames never match.
    prior_frame_faces: [u8; MAX_DICE],
    history: [u16; HISTORY_LEN],
    history_len: usize,
    rolls: u32,
    phase: DicePhase,
    rng: u32,
}

impl DiceGame {
    /// A new game with `DEFAULT_DICE` dice and one starting face each.
    #[must_use]
    pub fn new(seed: u32) -> Self {
        let mut game = Self {
            dice: DEFAULT_DICE,
            pending_dice: DEFAULT_DICE,
            faces: [0; MAX_DICE],
            final_faces: [0; MAX_DICE],
            prior_frame_faces: [0; MAX_DICE],
            history: [0; HISTORY_LEN],
            history_len: 0,
            rolls: 0,
            phase: DicePhase::Ready,
            rng: if seed == 0 { ZERO_SEED } else { seed },
        };
        for index in 0..game.dice {
            let face = game.roll_face();
            game.faces[index] = face;
            game.final_faces[index] = face;
        }
        game
    }

    #[must_use]
    pub const fn dice(&self) -> usize {
        self.dice
    }

    #[must_use]
    pub const fn phase(&self) -> DicePhase {
        self.phase
    }

    /// Faces on the table: the settled result, or the current tumble.
    #[must_use]
    pub fn faces(&self) -> &[u8] {
        &self.faces[..self.dice]
    }

    /// Faces the in-flight roll will land on. Meaningful only while
    /// `Rolling`; in `Ready` it matches `faces`.
    #[must_use]
    pub fn final_faces(&self) -> &[u8] {
        &self.final_faces[..self.dice]
    }

    /// Completed-roll totals, newest first.
    #[must_use]
    pub fn history(&self) -> &[u16] {
        &self.history[..self.history_len]
    }

    #[must_use]
    pub const fn rolls(&self) -> u32 {
        self.rolls
    }

    /// True on every tenth completed roll, when the caller should refresh
    /// the whole panel rather than the dirty dice area.
    #[must_use]
    pub const fn full_refresh_due(&self) -> bool {
        self.rolls > 0 && self.rolls % FULL_REFRESH_EVERY == 0
    }

    /// Start a roll from `Ready`, fixing the final faces at once. While
    /// rolling this changes nothing and returns `false`, so a shake cannot
    /// restart a roll.
    pub fn request_roll(&mut self) -> bool {
        if self.phase != DicePhase::Ready {
            return false;
        }
        for index in 0..self.dice {
            self.final_faces[index] = self.roll_face();
            self.prior_frame_faces[index] = 0;
        }
        self.phase = DicePhase::Rolling { frame: 1 };
        self.faces = self.tumbling_faces(1);
        true
    }

    /// One animation frame on. The last frame lands on the final faces,
    /// pushes the total onto the history and counts the roll. In `Ready`
    /// this does nothing and returns `false`.
    pub fn advance(&mut self) -> bool {
        let frame = match self.phase {
            DicePhase::Ready => return false,
            DicePhase::Rolling { frame } => frame,
        };
        if frame < FRAMES {
            self.phase = DicePhase::Rolling { frame: frame + 1 };
            self.faces = self.tumbling_faces(frame + 1);
            return true;
        }
        self.faces = self.final_faces;
        self.phase = DicePhase::Ready;
        self.rolls = self.rolls.saturating_add(1);
        self.push_history();
        self.apply_pending_dice();
        true
    }

    /// Choose how many dice to roll, clamped to `1..=MAX_DICE`. The new
    /// count takes effect from `Ready`: a change made mid-roll waits until
    /// that roll lands. The history is kept.
    pub fn set_dice(&mut self, dice: usize) {
        self.pending_dice = dice.clamp(1, MAX_DICE);
        if self.phase == DicePhase::Ready {
            self.apply_pending_dice();
        }
    }

    /// Total of the faces on the table. In `Ready` this is the last result.
    #[must_use]
    pub fn total(&self) -> u16 {
        self.faces().iter().map(|face| u16::from(*face)).sum()
    }

    fn apply_pending_dice(&mut self) {
        if self.pending_dice == self.dice {
            return;
        }
        let previous = self.dice;
        self.dice = self.pending_dice;
        // Keep the faces already on the table; deal only the new dice.
        for index in previous..self.dice {
            let face = self.roll_face();
            self.faces[index] = face;
            self.final_faces[index] = face;
        }
        for index in self.dice..MAX_DICE {
            self.faces[index] = 0;
            self.final_faces[index] = 0;
        }
    }

    fn push_history(&mut self) {
        let total = self.total();
        let keep = self.history_len.min(HISTORY_LEN - 1);
        self.history.copy_within(0..keep, 1);
        self.history[0] = total;
        self.history_len = (self.history_len + 1).min(HISTORY_LEN);
    }

    /// Faces for one animation frame: never the final faces (while a face
    /// is possible) and never the same set as the previous frame. The
    /// landing itself is `Ready`; every `Rolling` frame tumbles.
    fn tumbling_faces(&mut self, frame: u8) -> [u8; MAX_DICE] {
        let mut faces = [0; MAX_DICE];
        for index in 0..self.dice {
            faces[index] = self.roll_face();
        }
        if self.dice > 0 && SIDES > 1 {
            if faces == self.final_faces {
                faces[0] = self.other_face(faces[0], self.final_faces[0]);
            }
            if frame > 1 && faces == self.prior_frame_faces {
                faces[0] = self.other_face(faces[0], self.prior_frame_faces[0]);
            }
        }
        self.prior_frame_faces = faces;
        faces
    }

    /// A face that is not `avoid`, so two frames can differ in one place.
    fn other_face(&mut self, face: u8, avoid: u8) -> u8 {
        let mut candidate = self.roll_face();
        if candidate == avoid {
            candidate = if face == 1 { 2 } else { 1 };
            if candidate == avoid {
                candidate = if avoid == SIDES { 1 } else { avoid + 1 };
            }
        }
        candidate
    }

    /// One `1..=SIDES` face from the seeded xorshift.
    fn roll_face(&mut self) -> u8 {
        1 + (self.next_random() % u32::from(SIDES)) as u8
    }

    fn next_random(&mut self) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face_counts(game: &mut DiceGame) -> [u32; 7] {
        let mut counts = [0; 7];
        for _ in 0..6000 {
            assert!(game.request_roll());
            for _ in 0..FRAMES {
                assert!(game.advance());
            }
            for face in game.faces() {
                counts[*face as usize] += 1;
            }
        }
        counts
    }

    #[test]
    fn the_same_seed_rolls_the_same_faces() {
        let mut left = DiceGame::new(42);
        let mut right = DiceGame::new(42);
        for _ in 0..20 {
            assert!(left.request_roll());
            assert!(right.request_roll());
            assert_eq!(left.final_faces(), right.final_faces());
            for _ in 0..FRAMES {
                assert!(left.advance());
                assert!(right.advance());
            }
            assert_eq!(left.faces(), right.faces());
        }
    }

    #[test]
    fn one_die_fair_over_six_thousand_rolls() {
        let mut game = DiceGame::new(7);
        game.set_dice(1);
        let counts = face_counts(&mut game);
        let total: u32 = counts[1..].iter().sum();
        assert_eq!(total, 6000);
        for face in 1..=6 {
            let share = f64::from(counts[face]) / f64::from(total);
            assert!(
                (0.14..=0.19).contains(&share),
                "face {face} share {share:.3} outside 14%-19%"
            );
        }
    }

    #[test]
    fn two_dice_total_two_to_twelve_and_seven_is_most_common() {
        let mut game = DiceGame::new(99);
        game.set_dice(2);
        let mut totals = [0_u32; 13];
        for _ in 0..6000 {
            assert!(game.request_roll());
            for _ in 0..FRAMES {
                assert!(game.advance());
            }
            let total = usize::from(game.total());
            assert!((2..=12).contains(&total), "total {total} out of range");
            totals[total] += 1;
        }
        for total in 2..=12 {
            assert!(totals[total] > 0, "total {total} never came up");
        }
        assert_eq!(
            totals.iter().copied().max(),
            Some(totals[7]),
            "7 is the most common total"
        );
    }

    #[test]
    fn a_roll_takes_exactly_frames_advances() {
        let mut game = DiceGame::new(3);
        assert!(game.request_roll());
        for frame in 1..=FRAMES {
            assert_eq!(game.phase(), DicePhase::Rolling { frame });
            assert!(game.advance());
        }
        assert_eq!(game.phase(), DicePhase::Ready);
        assert_eq!(game.rolls(), 1);
    }

    #[test]
    fn request_roll_while_rolling_changes_nothing() {
        let mut game = DiceGame::new(11);
        assert!(game.request_roll());
        let phase = game.phase();
        let final_faces: Vec<u8> = game.final_faces().to_vec();
        let rng_before = game.rng;
        assert!(!game.request_roll());
        assert_eq!(game.phase(), phase);
        assert_eq!(game.final_faces(), final_faces.as_slice());
        assert_eq!(game.rng, rng_before);
    }

    #[test]
    fn set_dice_clamps_and_takes_effect_from_ready() {
        let mut game = DiceGame::new(5);
        assert_eq!(game.dice(), DEFAULT_DICE);
        game.set_dice(0);
        assert_eq!(game.dice(), 1);
        assert_eq!(game.faces().len(), 1);
        game.set_dice(9);
        assert_eq!(game.dice(), MAX_DICE);
        assert_eq!(game.faces().len(), MAX_DICE);

        // A change mid-roll waits for Ready.
        assert!(game.request_roll());
        game.set_dice(1);
        assert_eq!(game.dice(), MAX_DICE);
        for _ in 0..FRAMES {
            assert!(game.advance());
        }
        assert_eq!(game.phase(), DicePhase::Ready);
        assert_eq!(game.dice(), 1);
        assert_eq!(game.faces().len(), 1);
    }

    #[test]
    fn the_history_keeps_six_totals_newest_first() {
        let mut game = DiceGame::new(13);
        game.set_dice(1);
        let mut expected = Vec::new();
        for _ in 0..8 {
            assert!(game.request_roll());
            for _ in 0..FRAMES {
                assert!(game.advance());
            }
            expected.insert(0, game.total());
        }
        assert_eq!(game.history().len(), HISTORY_LEN);
        assert_eq!(game.history(), expected[..HISTORY_LEN].to_vec().as_slice());
    }

    #[test]
    fn a_full_refresh_is_due_on_the_tenth_roll() {
        let mut game = DiceGame::new(2);
        for roll in 1..=12 {
            assert!(game.request_roll());
            for _ in 0..FRAMES {
                assert!(game.advance());
            }
            assert_eq!(game.rolls(), roll);
            let due = roll % FULL_REFRESH_EVERY == 0;
            assert_eq!(
                game.full_refresh_due(),
                due,
                "roll {roll} full_refresh_due={}",
                game.full_refresh_due()
            );
        }
    }

    #[test]
    fn advance_in_ready_does_nothing() {
        let mut game = DiceGame::new(1);
        let before = game.clone();
        assert!(!game.advance());
        assert_eq!(game, before);
    }

    #[test]
    fn tumbling_frames_differ_from_the_result_and_each_other() {
        let mut game = DiceGame::new(21);
        game.set_dice(3);
        assert!(game.request_roll());
        let final_faces: Vec<u8> = game.final_faces().to_vec();
        let mut previous: Vec<u8> = game.faces().to_vec();
        assert_ne!(previous, final_faces, "frame 1 showed the result");
        for frame in 2..=FRAMES {
            assert!(game.advance());
            assert_eq!(game.phase(), DicePhase::Rolling { frame });
            let current: Vec<u8> = game.faces().to_vec();
            assert_ne!(current, previous, "frame {frame} repeated the prior");
            assert_ne!(current, final_faces, "frame {frame} showed the result");
            previous = current;
        }
        assert!(game.advance());
        assert_eq!(game.phase(), DicePhase::Ready);
        assert_eq!(game.faces(), final_faces.as_slice());
    }
}

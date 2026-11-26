use crate::repr::types::{BLACK, WHITE};
use crate::search::search_config::SearchMode;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeControl {
    Untimed,
    Rapid,
    Blitz,
    Bullet,
    Custom,
}

impl TimeControl {
    pub const OPTIONS: [Self; 5] = [
        Self::Untimed,
        Self::Rapid,
        Self::Blitz,
        Self::Bullet,
        Self::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Untimed => "Untimed",
            Self::Rapid => "10 min + 10 sec",
            Self::Blitz => "3 min + 3 sec",
            Self::Bullet => "1 min + 1 sec",
            Self::Custom => "Custom",
        }
    }

    pub fn durations(
        self,
        inputs: &[String; 3],
    ) -> Result<Option<(Duration, Duration)>, &'static str> {
        let (base, increment) = match self {
            Self::Untimed => return Ok(None),
            Self::Rapid => (600, 10),
            Self::Blitz => (180, 3),
            Self::Bullet => (60, 1),
            Self::Custom => {
                let parse = |s: &str| {
                    s.trim()
                        .parse::<u32>()
                        .map(u64::from)
                        .map_err(|_| "Enter whole, nonnegative minutes, seconds, and increment.")
                };
                let base = parse(&inputs[0])? * 60 + parse(&inputs[1])?;
                if base == 0 {
                    return Err("Base time must be greater than zero.");
                }
                (base, parse(&inputs[2])?)
            }
        };
        Ok(Some((
            Duration::from_secs(base),
            Duration::from_secs(increment),
        )))
    }
}

pub(super) struct Clocks {
    balances: [Duration; 2],
    increment: Duration,
    pub active: Option<(u32, Instant)>,
}

impl Clocks {
    pub fn new(base: Duration, increment: Duration, turn: Option<u32>, now: Instant) -> Self {
        Self {
            balances: [base; 2],
            increment,
            active: turn.map(|side| (side, now)),
        }
    }

    pub fn remaining(&self, side: u32, now: Instant) -> Duration {
        let mut balance = self.balances[side as usize];
        if let Some((active, start)) = self.active {
            if side == active {
                balance = balance.saturating_sub(now.saturating_duration_since(start));
            }
        }
        balance
    }

    pub fn check_flag(&mut self, now: Instant) -> bool {
        let Some((side, _)) = self.active else {
            return false;
        };
        if !self.remaining(side, now).is_zero() {
            return false;
        }
        self.balances[side as usize] = Duration::ZERO;
        self.active = None;
        true
    }

    pub fn flagged(&self) -> Option<u32> {
        [WHITE, BLACK]
            .into_iter()
            .find(|&side| self.balances[side as usize].is_zero())
    }

    pub fn finish_move(&mut self, now: Instant, game_over: bool) {
        if let Some((side, _)) = self.active {
            self.balances[side as usize] = self.remaining(side, now).saturating_add(self.increment);
            self.active = (!game_over).then_some((side ^ 1, now));
        }
    }

    pub fn search_mode(&self, turn: u32, now: Instant) -> SearchMode {
        let millis = |d: Duration| u64::try_from(d.as_millis()).unwrap_or(u64::MAX);
        let inc = millis(self.increment);
        SearchMode::time_control_with_margin(
            millis(self.remaining(WHITE, now)),
            millis(self.remaining(BLACK, now)),
            inc,
            inc,
            turn == WHITE,
        )
    }
}

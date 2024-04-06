#[doc(hidden)]
pub mod duration;
mod datetime;
#[doc(hidden)]
pub mod datetimeopt;

use std::time::Duration;

use chrono::{prelude::*, TimeDelta};
use serde::{Deserialize, Serialize};

/// Like a kitchen timer
#[derive(Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub struct Timer {
    #[serde(with = "crate::time::datetime::unix")]
    started_at: DateTime<Local>,
    #[serde(with = "crate::time::duration::seconds")]
    duration: Duration,
}

impl Timer {
    /// Create a new timer
    pub fn new(started_at: DateTime<Local>, duration: Duration) -> Self {
        Self {
            started_at,
            duration,
        }
    }

    /// Get the time this timer starts at
    pub fn starts_at(&self) -> DateTime<Local> {
        self.started_at
    }

    /// Get the time this timer ends at
    pub fn ends_at(&self) -> DateTime<Local> {
        self.started_at + self.duration
    }

    /// Get the length of time that this timer was set for
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Get the amount of time that has passed since this timer started
    pub fn elapsed(&self, now: DateTime<Local>) -> Duration {
        (now - self.started_at).clamp(TimeDelta::zero(), TimeDelta::from_std(self.duration).unwrap()).to_std().unwrap()
    }

    pub fn remaining(&self, now: DateTime<Local>) -> Duration {
        (self.duration - self.elapsed(now)).clamp(Duration::ZERO, self.duration)
    }

    /// Check if this timer's duration has run out
    pub fn done(&self, now: DateTime<Local>) -> bool {
        now > self.ends_at()
    }
}

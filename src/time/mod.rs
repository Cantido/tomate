mod datetime;
#[doc(hidden)]
pub mod datetimeopt;
#[doc(hidden)]
pub mod duration;

use chrono::{prelude::*, TimeDelta};
use serde::{Deserialize, Serialize};

/// Like a kitchen timer
#[derive(Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub struct Timer {
    #[serde(with = "crate::time::datetime::unix")]
    started_at: DateTime<Local>,
    #[serde(with = "crate::time::duration::seconds")]
    duration: TimeDelta,
}

impl Timer {
    /// Create a new timer
    pub fn new(started_at: DateTime<Local>, duration: TimeDelta) -> Self {
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
    pub fn duration(&self) -> TimeDelta {
        self.duration
    }

    /// Get the amount of time that has passed since this timer started
    pub fn elapsed(&self, now: DateTime<Local>) -> TimeDelta {
        (now - self.started_at).clamp(TimeDelta::zero(), self.duration)
    }

    /// Get the amount of time left on this timer
    pub fn remaining(&self, now: DateTime<Local>) -> TimeDelta {
        (self.duration - self.elapsed(now)).clamp(TimeDelta::zero(), self.duration)
    }

    /// Check if this timer's duration has run out
    pub fn done(&self, now: DateTime<Local>) -> bool {
        now > self.ends_at()
    }
}

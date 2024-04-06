#[doc(hidden)]
pub mod duration;
mod datetime;
#[doc(hidden)]
pub mod datetimeopt;

use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

/// Like a kitchen timer
#[derive(Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub struct Timer {
    #[serde(with = "crate::time::datetime::unix")]
    started_at: SystemTime,
    #[serde(with = "crate::time::duration::seconds")]
    duration: Duration,
}

impl Timer {
    /// Create a new timer
    pub fn new(started_at: SystemTime, duration: Duration) -> Self {
        Self {
            started_at,
            duration,
        }
    }

    /// Get the time this timer starts at
    pub fn starts_at(&self) -> SystemTime {
        self.started_at
    }

    /// Get the time this timer ends at
    pub fn ends_at(&self) -> SystemTime {
        self.started_at + self.duration
    }

    /// Get the length of time that this timer was set for
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Get the amount of time that has passed since this timer started
    pub fn elapsed(&self, now: SystemTime) -> Duration {
        if self.started_at < now {
            now.duration_since(self.started_at).unwrap().clamp(Duration::ZERO, self.duration)
        } else {
            Duration::ZERO
        }
    }

    /// Get the amount of time remaining before this timer expires
    pub fn remaining(&self, now: SystemTime) -> Duration {
        let elapsed = self.elapsed(now);

        if elapsed > self.duration {
            Duration::ZERO
        } else {
            (self.duration - elapsed).clamp(Duration::ZERO, self.duration)
        }
    }

    /// Check if this timer's duration has run out
    pub fn done(&self, now: SystemTime) -> bool {
        now > self.ends_at()
    }
}

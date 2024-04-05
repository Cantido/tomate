pub mod duration;
pub mod datetime;
pub mod datetimeopt;

use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Timer {
    #[serde(with = "crate::time::datetime::unix")]
    started_at: SystemTime,
    #[serde(with = "crate::time::duration::seconds")]
    duration: Duration,
}

impl Timer {
    pub fn new(started_at: SystemTime, duration: Duration) -> Self {
        Self {
            started_at,
            duration,
        }
    }

    pub fn starts_at(&self) -> SystemTime {
        self.started_at
    }

    pub fn ends_at(&self) -> SystemTime {
        self.started_at + self.duration
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn elapsed(&self, now: SystemTime) -> Duration {
        now.duration_since(self.started_at).unwrap()
    }

    pub fn remaining(&self, now: SystemTime) -> Duration {
        (self.duration - self.elapsed(now)).clamp(Duration::ZERO, self.duration)
    }

    pub fn done(&self, now: SystemTime) -> bool {
        now > self.ends_at()
    }
}

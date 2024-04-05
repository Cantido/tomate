pub mod duration;
pub mod datetime;
pub mod datetimeopt;

use std::time::Duration;

use chrono::{prelude::*, TimeDelta};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Timer {
    #[serde(with = "crate::time::datetime::unix")]
    started_at: DateTime<Local>,
    #[serde(with = "crate::time::duration::seconds")]
    duration: Duration,
}

impl Timer {
    pub fn new(started_at: DateTime<Local>, duration: Duration) -> Self {
        Self {
            started_at,
            duration,
        }
    }

    pub fn starts_at(&self) -> DateTime<Local> {
        self.started_at
    }

    pub fn ends_at(&self) -> DateTime<Local> {
        self.started_at + self.duration
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn elapsed(&self, now: DateTime<Local>) -> Duration {
        (now - self.started_at).clamp(TimeDelta::zero(), TimeDelta::from_std(self.duration).unwrap()).to_std().unwrap()
    }

    pub fn remaining(&self, now: DateTime<Local>) -> Duration {
        (self.duration - self.elapsed(now)).clamp(Duration::ZERO, self.duration)
    }

    pub fn done(&self, now: DateTime<Local>) -> bool {
        now > self.ends_at()
    }
}

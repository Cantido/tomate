use crate::time::Timer;
use chrono::{prelude::*, TimeDelta};
use serde::{Deserialize, Serialize};

/// A Pomodoro timer
#[derive(Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub struct Pomodoro {
    #[serde(flatten)]
    timer: Timer,
    description: Option<String>,
    tags: Option<Vec<String>>,
    #[serde(default, with = "crate::time::datetimeopt::unix")]
    finished_at: Option<DateTime<Local>>,
}

impl Pomodoro {
    /// Create a new timer
    pub fn new(starts_at: DateTime<Local>, duration: TimeDelta) -> Self {
        let timer = Timer::new(starts_at, duration);
        Self {
            timer,
            finished_at: None,
            description: None,
            tags: None,
        }
    }

    /// Get the struct describing the time this Pomodoro is running
    pub fn timer(&self) -> &Timer {
        &self.timer
    }

    /// Get the description
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Set the description
    pub fn set_description(&mut self, description: &str) {
        self.description = Some(description.to_string());
    }

    /// Get the tags
    pub fn tags(&self) -> Option<&Vec<String>> {
        self.tags.as_ref()
    }

    /// Set the tags
    pub fn set_tags(&mut self, tags: Vec<String>) {
        self.tags = Some(tags);
    }

    /// Stop running this timer
    pub fn finish(&mut self, now: DateTime<Local>) {
        self.finished_at = Some(now);
    }

    /// Get the duration that this Pomodoro lasted before it was finished.
    ///
    /// This is the actual time between start and finish. If you want to get
    /// the duration the timer was set for, use the duration of this Pomodoro's [`timer()`].
    pub fn duration(&self) -> Option<TimeDelta> {
        self.finished_at
            .map(|finished_at| finished_at - self.timer.starts_at())
    }
}

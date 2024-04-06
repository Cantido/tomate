use std::time::Duration;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use crate::time::Timer;

/// A Pomodoro timer
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pomodoro {
    #[serde(flatten)]
    timer: Timer,
    description: Option<String>,
    tags: Option<Vec<String>>,
    #[serde(default, with = "crate::time::datetimeopt::unix")]
    finished_at: Option<SystemTime>,
}

impl Pomodoro {
    /// Create a new timer
    pub fn new(starts_at: SystemTime, duration: Duration) -> Self {
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
    pub fn tags(&self) -> Option<&[String]> {
        self.tags.as_deref()
    }

    /// Set the tags
    pub fn set_tags(&mut self, tags: Vec<String>) {
        self.tags = Some(tags);
    }

    /// Stop running this timer
    pub fn finish(&mut self, now: SystemTime) {
        self.finished_at = Some(now);
    }
}

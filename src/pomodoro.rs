use std::time::Duration;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use crate::time::Timer;
use crate::time::duration;

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
    pub fn new(starts_at: SystemTime, duration: Duration) -> Self {
        let timer = Timer::new(starts_at, duration);
        Self {
            timer,
            finished_at: None,
            description: None,
            tags: None,
        }
    }

    pub fn timer(&self) -> &Timer {
        &self.timer
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn set_description(&mut self, description: &str) {
        self.description = Some(description.to_string());
    }

    pub fn tags(&self) -> Option<&[String]> {
        self.tags.as_deref()
    }

    pub fn set_tags(&mut self, tags: Vec<String>) {
        self.tags = Some(tags);
    }

    pub fn finish(&mut self, now: SystemTime) {
        self.finished_at = Some(now);
    }

    pub fn format(&self, f: &str, now: SystemTime) -> String {
        let output = f
            .replace("%d", &self.description.as_ref().unwrap_or(&"".to_string()))
            .replace(
                "%t",
                &self
                    .tags
                    .as_ref()
                    .unwrap_or(&Vec::<String>::new())
                    .join(","),
            )
            .replace("%r", &duration::to_kitchen(&self.timer.remaining(now)))
            .replace("%R", &self.timer.remaining(now).as_secs().to_string())
            .replace("%S", &self.timer.starts_at().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs().to_string())
            .replace("%E", &self.timer.ends_at().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs().to_string());

        output
    }
}

use chrono::{prelude::*, TimeDelta};
use serde::{Deserialize, Serialize};
use crate::time::{Timer, TimeDeltaExt};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pomodoro {
    #[serde(flatten)]
    timer: Timer,
    description: Option<String>,
    tags: Option<Vec<String>>,
    #[serde(default, with = "crate::time::datetimeopt::unix")]
    finished_at: Option<DateTime<Local>>,
}

impl Pomodoro {
    pub fn new(starts_at: DateTime<Local>, duration: TimeDelta) -> Self {
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

    pub fn finish(&mut self, now: DateTime<Local>) {
        self.finished_at = Some(now);
    }

    pub fn format(&self, f: &str, now: DateTime<Local>) -> String {
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
            .replace("%r", &self.timer.remaining(now).to_kitchen())
            .replace("%R", &self.timer.remaining(now).num_seconds().to_string())
            .replace("%s", &self.timer.starts_at().to_rfc3339())
            .replace("%S", &self.timer.starts_at().timestamp().to_string())
            .replace("%e", &self.timer.ends_at().to_rfc3339())
            .replace("%E", &self.timer.ends_at().timestamp().to_string());

        output
    }
}

//! Interact with Pomodoro timers

use crate::{hooks, time::Timer, Config, History, Status};
use anyhow::{anyhow, Context, Result};
use chrono::{prelude::*, TimeDelta};
use log::{info, warn};
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

/// Starts a new Pomodoro timer.
pub fn start(
    config: &Config,
    duration: &Option<TimeDelta>,
    description: &Option<String>,
    tags: &[String],
) -> Result<()> {
    let dur = duration.unwrap_or(config.pomodoro_duration);
    let timer_seconds = dur.num_seconds();

    let mut pom = Pomodoro::new(Local::now(), dur);
    if let Some(desc) = description {
        pom.set_description(desc);
    }

    pom.set_tags(tags.to_vec());

    let status = Status::load(&config.state_file_path)?;

    let start_result = match status {
        Status::ShortBreak(_timer) => Err(anyhow!("You're currently taking a break!")),
        Status::LongBreak(_timer) => Err(anyhow!("You're currently taking a break!")),
        Status::Active(_pom) => Err(anyhow!("There is already an unfinished Pomodoro")),
        Status::Inactive => {
            let next_status = Status::Active(pom);
            next_status
                .save(&config.state_file_path)
                .with_context(|| "Unable to save new Pomodoro")?;

            hooks::Hook::PomodoroStart
                .run(&config.hooks_directory)
                .with_context(|| "Failed to run pomodoro start hook")
        }
    };

    start_result?;

    let systemd_output = std::process::Command::new("systemd-run")
        .args([
            "--user".to_string(),
            format!("--on-active={}", timer_seconds),
            "--timer-property=AccuracySec=100ms".to_string(),
            std::env::current_exe()?.to_str().unwrap().to_string(),
            "timer".to_string(),
            "check".to_string(),
        ])
        .output()
        .with_context(|| "Failed to schedule systemd timer")?;

    if let Ok(output_msg) = String::from_utf8(systemd_output.stderr) {
        info!("{}", &output_msg);
    } else {
        warn!(
            "{}",
            "systemd-run printed bytes to stderr that were not valid UTF-8"
        );
    }

    Ok(())
}

/// Stop the current Pomodoro timer and log it to the history file.
pub fn stop(config: &Config) -> Result<()> {
    let status = Status::load(&config.state_file_path)?;

    if let Status::Active(mut pom) = status {
        hooks::Hook::PomodoroEnd.run(&config.hooks_directory)?;

        pom.finish(Local::now());

        History::append(&pom, &config.history_file_path)?;

        crate::clear(config)?;
    }

    Ok(())
}

use std::fs::{read_to_string, OpenOptions};
use std::io::prelude::*;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::{prelude::*, TimeDelta};
use colored::Colorize;
use log::info;
use serde::{Deserialize, Serialize};

use crate::Pomodoro;

/// A record of a past Pomodoro timer
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct HistoryEntry {
    #[serde(default, with = "crate::time::datetime::unix")]
    started_at: DateTime<Local>,
    #[serde(with = "crate::time::duration::seconds")]
    duration: TimeDelta,
    tags: Option<Vec<String>>,
    description: Option<String>,
}

impl HistoryEntry {
    pub fn archive(pom: &Pomodoro) -> Result<Self> {
        let duration = pom
            .duration()
            .with_context(|| "Pomodoro is not finished yet")?;

        Ok(Self {
            duration,
            started_at: pom.timer().starts_at(),
            tags: pom.tags().cloned(),
            description: pom.description().map(|s| s.to_owned()),
        })
    }
}

/// A record of past Pomodoro timers
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct History {
    pomodoros: Vec<Pomodoro>,
}

impl History {
    /// Load the history from a TOML file
    pub fn load(path: &Path) -> Result<Self> {
        if !path.try_exists()? {
            return Ok(Self::default());
        }

        let history_str = read_to_string(path).with_context(|| "Failed to read history file")?;
        toml::from_str(&history_str).with_context(|| "Failed to parse history file")
    }

    /// Get the list of historical Pomodoros
    pub fn pomodoros(&self) -> &Vec<Pomodoro> {
        &self.pomodoros
    }

    /// Append a new Pomodoro to a history file
    pub fn append(pomodoro: &Pomodoro, history_file_path: &Path) -> Result<()> {
        info!(
            "Archiving Pomodoro to {}",
            &history_file_path.display().to_string().cyan()
        );

        std::fs::create_dir_all(
            history_file_path
                .parent()
                .with_context(|| "History file path does not have a parent directory")?,
        )?;

        let mut history_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(history_file_path)?;

        let entry = HistoryEntry::archive(pomodoro)?;

        let pom_str = toml::to_string(&entry)?;
        writeln!(history_file, "[[pomodoros]]\n{}", pom_str)?;

        Ok(())
    }
}

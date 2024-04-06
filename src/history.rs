use std::fs::{
    OpenOptions,
    read_to_string
};
use std::io::prelude::*;
use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::Pomodoro;

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

        let history_str = read_to_string(path)
            .with_context(|| "Failed to read history file")?;
        toml::from_str(&history_str)
            .with_context(|| "Failed to parse history file")
    }

    /// Get the list of historical Pomodoros
    pub fn pomodoros(&self) -> &Vec<Pomodoro> {
        &self.pomodoros
    }

    /// Append a new Pomodoro to a history file
    pub fn append(pomodoro: &Pomodoro, history_file_path: &Path) -> Result<()> {
        println!(
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
            .write(true)
            .append(true)
            .open(&history_file_path)?;

        let pom_str = toml::to_string(&pomodoro)?;
        writeln!(history_file, "[[pomodoros]]\n{}", pom_str)?;

        Ok(())
    }
}

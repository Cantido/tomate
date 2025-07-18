#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

//! A Pomodoro CLI and library
//!
//! Manages a set of files that describe the current Pomodoro or break,
//! as well as a history of completed Pomodoros.
//!
//! All the interface functions in this module require a [`Config`] struct.
//! Check out that struct's documentation for default values and functions
//! for loading and saving a configuration.

use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::Path,
};

use anyhow::{bail, Context, Result};
use chrono::prelude::*;
use colored::Colorize;
use log::info;
use serde::{Deserialize, Serialize};

mod config;
pub use config::{default_config_path, Config};
mod history;
pub use history::History;
mod hooks;
pub mod pomodoro;
pub use pomodoro::Pomodoro;
pub mod long_break;
pub mod short_break;
mod time;
pub use time::Timer;

/// Phases of the Pomodoro technique
#[derive(Clone, Eq, PartialEq, Hash, Debug, Deserialize, Serialize)]
pub enum Status {
    /// No Pomodoro or break is active
    Inactive,
    /// A Pomodoro is active
    Active(Pomodoro),
    /// A timer for a short break is active
    ShortBreak(Timer),
    /// A timer for a long break is active
    LongBreak(Timer),
}

impl Status {
    /// Load from a state file
    pub fn load(state_file_path: &Path) -> Result<Self> {
        if state_file_path.try_exists()? {
            let file = OpenOptions::new().read(true).open(state_file_path)?;
            Self::from_reader(file)
        } else {
            Ok(Self::Inactive)
        }
    }

    /// Load state from a reader
    pub fn from_reader<R>(reader: R) -> Result<Self>
    where
        R: Read,
    {
        let state_str =
            std::io::read_to_string(reader).with_context(|| "Failed to read state file")?;

        toml::from_str(&state_str).with_context(|| "Failed to parse state file")
    }

    /// Save this status as a TOML file
    pub fn save(&self, state_file_path: &Path) -> Result<()> {
        match &self {
            Self::Inactive => {
                info!(
                    "Deleting current Pomodoro state file {}",
                    &state_file_path.display().to_string().cyan()
                );
                std::fs::remove_file(state_file_path)?;
                Ok(())
            }
            _ => {
                if !state_file_path.try_exists()? {
                    info!(
                        "Creating Pomodoro state file {}",
                        &state_file_path.display().to_string().cyan()
                    );
                }

                let state_file_dir = state_file_path
                    .parent()
                    .with_context(|| "State file path does not have a parent directory")?;
                std::fs::create_dir_all(state_file_dir)
                    .with_context(|| "Failed to create directory for state file")?;

                let file = OpenOptions::new()
                    .create(true)
                    .read(true)
                    .write(true)
                    .truncate(true)
                    .open(state_file_path)
                    .with_context(|| {
                        format!("Unable to open state file {}", state_file_path.display())
                    })?;

                self.to_writer(file).with_context(|| {
                    format!("Failed to save Pomodoro to {}", state_file_path.display())
                })?;

                Ok(())
            }
        }
    }

    /// Save this pomodoro to an output stream
    pub fn to_writer<W>(&self, mut writer: W) -> Result<()>
    where
        W: Write,
    {
        let contents = toml::to_string(&self).with_context(|| "Unable to serialize Pomodoro")?;

        writer
            .write_all(contents.as_bytes())
            .with_context(|| "Unable to save Pomodoro to writer")
    }
}

/// Finish and archive a Pomodoro or break timer
pub fn finish(config: &Config) -> Result<()> {
    let status = Status::load(&config.state_file_path)?;

    match status {
        Status::Inactive => bail!("No active Pomodoro. Start one with \"tomate start\""),
        Status::ShortBreak(_timer) => {
            hooks::Hook::ShortBreakEnd.run(&config.hooks_directory)?;

            clear(config)?
        }
        Status::LongBreak(_timer) => {
            hooks::Hook::LongBreakEnd.run(&config.hooks_directory)?;

            clear(config)?
        }
        Status::Active(mut pom) => {
            hooks::Hook::PomodoroEnd.run(&config.hooks_directory)?;

            pom.finish(Local::now());

            History::append(&pom, &config.history_file_path)?;

            clear(config)?;
        }
    }

    Ok(())
}

/// Clear the current state by deleting the state file
pub fn clear(config: &Config) -> Result<()> {
    let state_file_path = &config.state_file_path;

    if state_file_path.exists() {
        info!(
            "Deleting current Pomodoro state file {}",
            &config.state_file_path.display().to_string().cyan()
        );
        std::fs::remove_file(&config.state_file_path)?;
    }

    Ok(())
}

/// Delete the state and history files
pub fn purge(config: &Config) -> Result<()> {
    if config.state_file_path.exists() {
        info!(
            "Removing current Pomodoro file at {}",
            config.state_file_path.display().to_string().cyan()
        );
        std::fs::remove_file(&config.state_file_path)?;
    }

    if config.history_file_path.exists() {
        info!(
            "Removing history file at {}",
            config.history_file_path.display().to_string().cyan()
        );
        std::fs::remove_file(&config.history_file_path)?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use chrono::{prelude::*, TimeDelta};

    use crate::{Pomodoro, Status};

    #[test]
    fn status_to_toml() {
        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        let mut pom = Pomodoro::new(dt, dur);
        pom.set_description("test converting poms to toml");
        pom.set_tags(vec!["test".to_string(), "toml".to_string()]);

        let status = Status::Active(pom);

        let toml = toml::to_string(&status).unwrap();
        let lines: Vec<&str> = toml.lines().collect();

        assert_eq!(lines[0], "[Active]");

        assert_eq!(lines[1], "started_at = 1711562400");
        assert_eq!(lines[2], "duration = 1500");
        assert_eq!(lines[3], r#"description = "test converting poms to toml""#);
        assert_eq!(lines[4], r#"tags = ["test", "toml"]"#);
    }

    #[test]
    fn toml_to_pom() {
        let pom: Pomodoro = toml::from_str(
            r#"
started_at = 1712346817
duration = 1500
description = "Do something cool"
tags = ["work", "fun"]
            "#,
        )
        .expect("Could not parse pomodoro from string");

        let dt: DateTime<Local> = "2024-04-05T13:53:37-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        assert_eq!(pom.timer().starts_at(), dt);
        assert_eq!(pom.timer().duration(), dur);
        assert_eq!(pom.description(), Some("Do something cool"));
        let tags = vec!["work".to_string(), "fun".to_string()];
        assert_eq!(pom.tags().unwrap(), &tags);
    }

    #[test]
    fn time_elapsed() {
        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dt_later: DateTime<Local> = "2024-03-27T12:20:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        let pom = Pomodoro::new(dt, dur);

        let expected_elapsed = TimeDelta::new(20 * 60, 0).unwrap();

        assert_eq!(pom.timer().elapsed(dt_later), expected_elapsed);
    }

    #[test]
    fn time_remaining() {
        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dt_later: DateTime<Local> = "2024-03-27T12:20:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        let pom = Pomodoro::new(dt, dur);

        let expected_remaining = TimeDelta::new(5 * 60, 0).unwrap();

        assert_eq!(pom.timer().remaining(dt_later), expected_remaining);
    }
}

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

use std::{fs::OpenOptions, io::{Read, Write}, path::Path, time::SystemTime};

use anyhow::{anyhow, bail, Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};

mod config;
pub use config::{Config, default_config_path};
mod history;
pub use history::History;
mod hooks;
mod pomodoro;
pub use pomodoro::Pomodoro;
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
        R: Read
    {
        let state_str = std::io::read_to_string(reader)
            .with_context(|| "Failed to read state file")?;

        toml::from_str(&state_str)
            .with_context(|| "Failed to parse state file")
    }

    /// Save this status as a TOML file
    pub fn save(&self, state_file_path: &Path) -> Result<()> {
        match &self {
            Self::Inactive => {
                println!(
                    "Deleting current Pomodoro state file {}",
                    &state_file_path.display().to_string().cyan()
                );
                std::fs::remove_file(&state_file_path)?;
                Ok(())
            },
            _ => {
                if !state_file_path.try_exists()? {
                    println!(
                        "Creating Pomodoro state file {}",
                        &state_file_path.display().to_string().cyan()
                    );
                }

                let state_file_dir = state_file_path.parent()
                        .with_context(|| "State file path does not have a parent directory")?;
                std::fs::create_dir_all(state_file_dir)
                    .with_context(|| "Failed to create directory for state file")?;

                let file = OpenOptions::new().read(true).write(true).truncate(true).open(state_file_path)?;
                self.to_writer(file)?;

                Ok(())
            },
        }
    }

    /// Save this pomodoro to an output stream
    pub fn to_writer<W>(&self, mut writer: W) -> Result<()>
    where
        W: Write
    {
        let contents = toml::to_string(&self)
            .with_context(|| "Unable to serialize Pomodoro")?;

        writer.write_all(&contents.as_bytes())
            .with_context(|| "Unable to save Pomodoro to writer")
    }
}

/// Start a Pomodoro timer
pub fn start(config: &Config, pomodoro: Pomodoro) -> Result<Status> {
    let status = Status::load(&config.state_file_path)?;

    match status {
        Status::ShortBreak(_timer) => Err(anyhow!("You're currently taking a break!")),
        Status::LongBreak(_timer) => Err(anyhow!("You're currently taking a break!")),
        Status::Active(_pom) => Err(anyhow!("There is already an unfinished Pomodoro")),
        Status::Inactive => {
            let next_status = Status::Active(pomodoro);
            next_status.save(&config.state_file_path)?;

            hooks::run_start_hook(&config.hooks_directory)?;

            Ok(next_status)
        }
    }
}

/// Start a short break timer
pub fn take_short_break(config: &Config, timer: Timer) -> Result<()> {
    let status = Status::load(&config.state_file_path)?;

    match status {
        Status::Active(_) => Err(anyhow!("Finish your current timer before taking a break")),
        Status::ShortBreak(_) => Err(anyhow!("You are already taking a break")),
        Status::LongBreak(_) => Err(anyhow!("You are already taking a break")),
        Status::Inactive => {
            let new_status = Status::ShortBreak(timer.clone());
            new_status.save(&config.state_file_path)?;

            hooks::run_break_hook(&config.hooks_directory)?;

            Ok(())
        }
    }
}

/// Start a long break timer
pub fn take_long_break(config: &Config, timer: Timer) -> Result<()> {
    let status = Status::load(&config.state_file_path)?;

    match status {
        Status::Active(_) => Err(anyhow!("Finish your current timer before taking a break")),
        Status::ShortBreak(_) => Err(anyhow!("You are already taking a break")),
        Status::LongBreak(_) => Err(anyhow!("You are already taking a break")),
        Status::Inactive => {
            let new_status = Status::ShortBreak(timer.clone());
            new_status.save(&config.state_file_path)?;

            hooks::run_break_hook(&config.hooks_directory)?;

            Ok(())
        }
    }
}

/// Finish and archive a Pomodoro or break timer
pub fn finish(config: &Config) -> Result<()> {
    let status = Status::load(&config.state_file_path)?;

    match status {
        Status::Inactive => bail!("No active Pomodoro. Start one with \"tomate start\""),
        Status::ShortBreak(_timer) => clear(config)?,
        Status::LongBreak(_timer) => clear(config)?,
        Status::Active(mut pom) => {
            pom.finish(SystemTime::now());

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
        println!(
            "Deleting current Pomodoro state file {}",
            &config.state_file_path.display().to_string().cyan()
        );
        std::fs::remove_file(&config.state_file_path)?;

        hooks::run_stop_hook(&config.hooks_directory)?;
    }

    Ok(())
}

/// Delete the state and history files
pub fn purge(config: &Config) -> Result<()> {
    if config.state_file_path.exists() {
        println!(
            "Removing current Pomodoro file at {}",
            config.state_file_path.display().to_string().cyan()
        );
        std::fs::remove_file(&config.state_file_path)?;
    }

    if config.history_file_path.exists() {
        println!(
            "Removing history file at {}",
            config.history_file_path.display().to_string().cyan()
        );
        std::fs::remove_file(&config.history_file_path)?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use std::time::{Duration, SystemTime};

    use crate::{Pomodoro, Status};

    #[test]
    fn status_to_toml() {
        let dt: SystemTime = SystemTime::UNIX_EPOCH + Duration::from_secs(1711562400);
        let dur = Duration::new(25 * 60, 0);

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
        let pom: Pomodoro = toml::from_str(r#"
started_at = 1712346817
duration = 1500
description = "Do something cool"
tags = ["work", "fun"]
            "#,
        )
        .expect("Could not parse pomodoro from string");

        let dt: SystemTime = SystemTime::UNIX_EPOCH + Duration::from_secs(1712346817);
        let dur = Duration::new(25 * 60, 0);

        assert_eq!(pom.timer().starts_at(), dt);
        assert_eq!(pom.timer().duration(), dur);
        assert_eq!(pom.description(), Some("Do something cool"));
        let tags = vec!["work".to_string(), "fun".to_string()];
        assert_eq!(pom.tags().unwrap(), tags);
    }

    #[test]
    fn time_elapsed() {
        let dt: SystemTime = SystemTime::UNIX_EPOCH + Duration::from_secs(1711562400);
        let dt_later: SystemTime = SystemTime::UNIX_EPOCH + Duration::from_secs(1711562400 + (20 * 60));
        let dur = Duration::new(25 * 60, 0);

        let pom = Pomodoro::new(dt, dur);

        let expected_elapsed = Duration::new(20 * 60, 0);

        assert_eq!(pom.timer().elapsed(dt_later), expected_elapsed);
    }

    #[test]
    fn time_remaining() {
        let dt: SystemTime = SystemTime::UNIX_EPOCH + Duration::from_secs(1711562400);
        let dt_later: SystemTime = SystemTime::UNIX_EPOCH + Duration::from_secs(1711562400 + (20 * 60));
        let dur = Duration::new(25 * 60, 0);

        let pom = Pomodoro::new(dt, dur);

        let expected_remaining = Duration::new(5 * 60, 0);

        assert_eq!(pom.timer().remaining(dt_later), expected_remaining);
    }
}

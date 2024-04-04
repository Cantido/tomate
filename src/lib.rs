use std::{fs::read_to_string, path::PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{prelude::*, TimeDelta};
use colored::Colorize;
use config::Config;
use directories::ProjectDirs;
use history::History;
use serde::{Deserialize, Serialize};
use time::{Timer, TimeDeltaExt};

pub mod config;
pub mod history;
pub mod hooks;
pub mod time;

#[derive(Debug, Deserialize, Serialize)]
pub enum Status {
    Inactive,
    Active(Pomodoro),
    ShortBreak(Timer),
    LongBreak(Timer),
}

impl Status {
    pub fn load(state_file_path: &PathBuf) -> Result<Self> {
        if state_file_path.try_exists()? {
            let state_str = read_to_string(state_file_path)
                .with_context(|| "Failed to read state file")?;
            toml::from_str(&state_str)
                .with_context(|| "Failed to parse state file")
        } else {
            Ok(Self::Inactive)
        }
    }

    pub fn save(&self, state_file_path: &PathBuf) -> Result<()> {
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

                let contents = toml::to_string(&self)?;

                std::fs::write(&state_file_path, contents)?;

                Ok(())
            },
        }
    }

    pub fn timer(&self) -> Option<Timer> {
        match self {
            Status::Inactive => None,
            Status::Active(pom) => Some(pom.timer().clone()),
            Status::ShortBreak(timer) => Some(timer.clone()),
            Status::LongBreak(timer) => Some(timer.clone()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pomodoro {
    timer: Timer,
    description: Option<String>,
    tags: Option<Vec<String>>,
}

impl Pomodoro {
    pub fn new(starts_at: DateTime<Local>, duration: TimeDelta) -> Self {
        let timer = Timer::new(starts_at, duration);
        Self {
            timer,
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

pub fn default_config_path() -> Result<PathBuf> {
    let conf_path = ProjectDirs::from("dev", "Cosmicrose", "Tomate")
        .with_context(|| "Unable to determine XDG directories")?
        .config_dir()
        .join("config.toml");

    Ok(conf_path)
}

pub fn finish(config: &Config) -> Result<()> {
    let status = Status::load(&config.state_file_path)?;

    match status {
        Status::Inactive => bail!("No active Pomodoro. Start one with \"tomate start\""),
        Status::ShortBreak(_timer) => clear(config)?,
        Status::LongBreak(_timer) => clear(config)?,
        Status::Active(pom) => {
            History::append(&pom, &config.history_file_path)?;

            clear(config)?;
        }
    }

    Ok(())
}

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
    use chrono::{prelude::*, TimeDelta};

    use crate::{Pomodoro, Status};

    #[test]
    fn status_to_toml() {
        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        let pom = Pomodoro::new(dt, dur);

        let status = Status::Active(pom);

        let toml = toml::to_string_pretty(&status).unwrap();
        let lines: Vec<&str> = toml.lines().collect();

        assert_eq!(lines[0], "status = \"Active\"");
        assert_eq!(lines[1], "timer = \"2024-03-27T12:00:00-06:00/PT1500S\"");
    }

    #[test]
    fn toml_to_pom() {
        let pom: Pomodoro = toml::from_str(
            r#"timer = "2024-03-27T12:00:00-06:00/PT1500S""#,
        )
        .expect("Could not parse pomodoro from string");

        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        assert_eq!(pom.timer().starts_at(), dt);
        assert_eq!(pom.timer().duration(), dur);
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


    #[test]
    fn pomodoro_format_wallclock() {
        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        let pom = Pomodoro::new(dt, dur);

        let actual_format = pom.format("%r", dt);

        assert_eq!(actual_format, "25:00");
    }

    #[test]
    fn pomodoro_format_description() {
        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        let mut pom = Pomodoro::new(dt, dur);
        pom.set_description("hello :)");

        let actual_format = pom.format("%d", dt);

        assert_eq!(actual_format, "hello :)");
    }

    #[test]
    fn pomodoro_format_remaining() {
        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        let pom = Pomodoro::new(dt, dur);

        let actual_format = pom.format("%R", dt);

        assert_eq!(actual_format, "1500");
    }

    #[test]
    fn pomodoro_format_start_iso() {
        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        let pom = Pomodoro::new(dt, dur);

        let actual_format = pom.format("%s", dt);

        assert_eq!(actual_format, "2024-03-27T12:00:00-06:00");
    }

    #[test]
    fn pomodoro_format_start_timestamp() {
        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        let pom = Pomodoro::new(dt, dur);

        let actual_format = pom.format("%S", dt);

        assert_eq!(actual_format, "1711562400");
    }

    #[test]
    fn pomodoro_format_tags() {
        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        let mut pom = Pomodoro::new(dt, dur);
        pom.set_tags(vec!["a".to_string(), "b".to_string(), "c".to_string()]);

        let actual_format = pom.format("%t", dt);

        assert_eq!(actual_format, "a,b,c");
    }

    #[test]
    fn pomodoro_format_eta() {
        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        let pom = Pomodoro::new(dt, dur);

        let actual_format = pom.format("%e", dt);

        assert_eq!(actual_format, "2024-03-27T12:25:00-06:00");
    }

    #[test]
    fn pomodoro_format_eta_timestamp() {
        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        let pom = Pomodoro::new(dt, dur);

        let actual_format = pom.format("%E", dt);

        assert_eq!(actual_format, "1711563900");
    }
}

use std::{fs::read_to_string, path::{Path, PathBuf}, time::Duration};

use anyhow::{Context, Result};
use colored::Colorize;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

/// Global configuration values
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    /// Directory to find hook executables
    pub hooks_directory: PathBuf,
    /// File describing the current Pomodoro or break timer
    pub state_file_path: PathBuf,
    /// File describing historical Pomodoro or break timers
    pub history_file_path: PathBuf,
    /// Default duration for Pomodoro timers
    #[serde(with = "crate::time::duration::seconds")]
    pub pomodoro_duration: Duration,
    /// Default duration for short break timers
    #[serde(with = "crate::time::duration::seconds")]
    pub short_break_duration: Duration,
    /// Default duration for long break timers
    #[serde(with = "crate::time::duration::seconds")]
    pub long_break_duration: Duration,
}

impl Config {
    /// Returns the current config, creating a default config file if one does not exist
    pub fn init(config_path: &Path) -> Result<Self> {
        if let Some(conf) = Config::load(&config_path)? {
            Ok(conf)
        } else {
            let conf = Config::default();

            println!(
                "Creating config file at {}",
                config_path.display().to_string().cyan()
            );
            std::fs::write(&config_path, toml::to_string(&conf)?)?;

            Ok(conf)
        }
    }

    /// Reads a TOML config file
    pub fn load(path: &Path) -> Result<Option<Self>> {
        if path.exists() {
            let config_str = read_to_string(path)?;

            toml::from_str(&config_str).with_context(|| "Failed to parse config from TOML")
        } else {
            Ok(None)
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let project_dirs = ProjectDirs::from("dev", "Cosmicrose", "Tomate")
            .with_context(|| "Unable to determine XDG directories")
            .unwrap();

        let hooks_directory = project_dirs.config_dir().join("hooks");

        let state_file_path = project_dirs
            .state_dir()
            .with_context(|| "Getting state dir")
            .unwrap()
            .join("current.toml");

        let history_file_path = project_dirs.data_dir().join("history.toml");

        Self {
            hooks_directory,
            state_file_path,
            history_file_path,
            pomodoro_duration: Duration::new(25 * 60, 0),
            short_break_duration: Duration::new(5 * 60, 0),
            long_break_duration: Duration::new(30 * 60, 0),
        }
    }
}


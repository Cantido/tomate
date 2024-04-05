use std::{fs::read_to_string, path::{Path, PathBuf}, time::Duration};

use anyhow::{Context, Result};
use colored::Colorize;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub hooks_directory: PathBuf,
    pub state_file_path: PathBuf,
    pub history_file_path: PathBuf,
    #[serde(with = "crate::time::duration::seconds")]
    pub pomodoro_duration: Duration,
    #[serde(with = "crate::time::duration::seconds")]
    pub short_break_duration: Duration,
    #[serde(with = "crate::time::duration::seconds")]
    pub long_break_duration: Duration,
}

impl Config {
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

    fn load(path: &Path) -> Result<Option<Self>> {
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


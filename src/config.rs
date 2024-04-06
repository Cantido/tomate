use std::{fs::read_to_string, path::{Path, PathBuf}, time::Duration};

use anyhow::{Context, Result};
use colored::Colorize;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

/// Global configuration values
///
/// Tomate's configuration is stored in a TOML file in the current user's
/// config directory, which is `~/.config/tomate/config.toml` by default.
///
/// A Tomate config can be loaded from a file with [`Config::load`].
/// You can also use [`Config::init`] or [`Config::init_default`] to create
/// a default config file if one does not exist at the given path.
///
/// To save a config to the filesystem, use [`Config::save`].
///
/// ## File Format
///
/// The configuration file is written as a TOML file.
/// See the documentation for each field to learn how they are serialized.
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    /// Directory to find hook executables
    ///
    /// Default is a directory called `hooks` inside the config directory.
    /// Serialized as an absolute path.
    #[serde(default = "default_hooks_directory")]
    pub hooks_directory: PathBuf,
    /// File describing the current Pomodoro or break timer
    ///
    /// Default location is the user's state directory,
    /// which is usually `~/.local/state/tomate/current.toml`.
    /// Serialized as an absolute path.
    #[serde(default = "default_state_path")]
    pub state_file_path: PathBuf,
    /// File describing historical Pomodoro or break timers
    ///
    /// Default location is the user's data directory,
    /// which is usually `~/.local/share/tomate/history.toml`.
    /// Serialized as an absolute path.
    #[serde(default = "default_history_path")]
    pub history_file_path: PathBuf,
    /// Default duration for Pomodoro timers
    ///
    /// Default is 25 minutes (1500 seconds).
    /// Serialized as an integer count of seconds.
    #[serde(default = "default_pomodoro_duration", with = "crate::time::duration::seconds")]
    pub pomodoro_duration: Duration,
    /// Default duration for short break timers
    ///
    /// Default is 5 minutes (300 seconds).
    /// Serialized as an integer count of seconds.
    #[serde(default = "default_short_break_duration", with = "crate::time::duration::seconds")]
    pub short_break_duration: Duration,
    /// Default duration for long break timers
    ///
    /// Default is 20 minutes (1200 seconds).
    /// Serialized as an integer count of seconds.
    #[serde(default = "default_long_break_duration", with = "crate::time::duration::seconds")]
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

            conf.save(&config_path)?;

            Ok(conf)
        }
    }

    /// Returns the current config from the default location, and creates the file if one does not exist
    pub fn init_default() -> Result<Self> {
        let path = crate::default_config_path()?;
        Self::init(&path)
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

    /// Write this config file to the filesystem
    pub fn save(&self, path: &Path) -> Result<()> {
        let toml = toml::to_string(&self)
            .with_context(|| "Unable to format config as TOML")?;

        std::fs::write(&path, toml)
            .with_context(|| format!("Unable to write config TOML to path {}", path.display()))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hooks_directory: default_hooks_directory(),
            state_file_path: default_state_path(),
            history_file_path: default_history_path(),
            pomodoro_duration: default_pomodoro_duration(),
            short_break_duration: default_short_break_duration(),
            long_break_duration: default_long_break_duration(),
        }
    }
}

/// Get the default location of the config file
pub fn default_config_path() -> Result<PathBuf> {
    let conf_path = ProjectDirs::from("dev", "Cosmicrose", "Tomate")
        .with_context(|| "Unable to determine XDG directories")?
        .config_dir()
        .join("config.toml");

    Ok(conf_path)
}

fn default_hooks_directory() -> PathBuf {
    let project_dirs = ProjectDirs::from("dev", "Cosmicrose", "Tomate")
        .with_context(|| "Unable to determine XDG directories")
        .unwrap();

    project_dirs.config_dir().join("hooks")
}

fn default_state_path() -> PathBuf {
    ProjectDirs::from("dev", "Cosmicrose", "Tomate")
        .with_context(|| "Unable to determine XDG directories")
        .unwrap()
        .state_dir()
        .with_context(|| "Getting state dir")
        .unwrap()
        .join("current.toml")
}

fn default_history_path() -> PathBuf {
    ProjectDirs::from("dev", "Cosmicrose", "Tomate")
        .with_context(|| "Unable to determine XDG directories")
        .unwrap()
        .data_dir()
        .join("history.toml")
}

fn default_pomodoro_duration() -> Duration {
    Duration::from_secs(25 * 60)
}

fn default_short_break_duration() -> Duration {
    Duration::from_secs(5 * 60)
}

fn default_long_break_duration() -> Duration {
    Duration::from_secs(20 * 60)
}



use std::{fs::read_to_string, path::PathBuf};

use anyhow::{Context, Result};
use chrono::TimeDelta;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};


#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
  pub hooks_directory: PathBuf,
  pub state_file_path: PathBuf,
  pub history_file_path: PathBuf,
  #[serde(with = "crate::time::duration")]
  pub pomodoro_duration: TimeDelta,
  #[serde(with = "crate::time::duration")]
  pub short_break_duration: TimeDelta,
}

impl Config {
  pub fn load(path: &PathBuf) -> Result<Option<Self>> {
    if path.exists() {
      let config_str = read_to_string(path)?;

      toml::from_str(&config_str)
        .with_context(|| "Failed to parse config from TOML")
    } else {
      Ok(None)
    }
  }
}

impl Default for Config {
  fn default() -> Self {
      let project_dirs =
        ProjectDirs::from("dev", "Cosmicrose", "Tomate")
        .with_context(|| "Unable to determine XDG directories").unwrap();

      let hooks_directory =
        project_dirs
        .config_dir()
        .join("hooks");

      let state_file_path =
        project_dirs
        .state_dir()
        .with_context(|| "Getting state dir").unwrap()
        .join("current.toml");

      let history_file_path =
        project_dirs
        .data_dir()
        .join("history.toml");

      Self {
        hooks_directory,
        state_file_path,
        history_file_path,
        pomodoro_duration: TimeDelta::new(25 * 60, 0).unwrap(),
        short_break_duration: TimeDelta::new(5 * 60, 0).unwrap(),
      }
  }
}

pub fn default_config_path() -> Result<PathBuf> {
  let conf_path =
    ProjectDirs::from("dev", "Cosmicrose", "Tomate")
    .with_context(|| "Unable to determine XDG directories")?
    .config_dir()
    .join("config.toml");

  Ok(conf_path)
}

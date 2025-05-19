use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;
use log::info;

pub enum Hook {
    PomodoroStart,
    PomodoroEnd,
    ShortBreakStart,
    ShortBreakEnd,
    LongBreakStart,
    LongBreakEnd,
}

impl Hook {
    pub fn run(&self, hooks_directory: &Path) -> Result<()> {
        let hook_file_name = match *self {
            Self::PomodoroStart => "pomodoro-start",
            Self::PomodoroEnd => "pomodoro-end",
            Self::ShortBreakStart => "shortbreak-start",
            Self::ShortBreakEnd => "shortbreak-end",
            Self::LongBreakStart => "longbreak-start",
            Self::LongBreakEnd => "longbreak-end",
        };

        let hook_path = hooks_directory.join(hook_file_name);

        if hook_path.exists() {
            info!(
                "Executing hook at {}",
                hook_path.display().to_string().cyan()
            );

            std::process::Command::new(hook_path)
                .output()
                .with_context(|| "Failed to execute hook")?;
        }

        Ok(())
    }
}

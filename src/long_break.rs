//! Interact with long break timers

use std::io::{self, Write};

use anyhow::{anyhow, Context, Result};
use chrono::{Local, TimeDelta};

use crate::{hooks::Hook, Config, Status, Timer};

/// Start a long break timer
pub fn start(config: &Config, duration: &Option<TimeDelta>) -> Result<()> {
    let dur = duration.unwrap_or(config.long_break_duration);
    let timer = Timer::new(Local::now(), dur);

    let status = Status::load(&config.state_file_path)?;

    let result = match status {
        Status::Active(_) => Err(anyhow!("Finish your current timer before taking a break")),
        Status::ShortBreak(_) => Err(anyhow!("You are already taking a break")),
        Status::LongBreak(_) => Err(anyhow!("You are already taking a break")),
        Status::Inactive => {
            let new_status = Status::LongBreak(timer.clone());
            new_status.save(&config.state_file_path)?;

            Hook::LongBreakStart.run(&config.hooks_directory)?;

            Ok(())
        }
    };

    result?;

    let systemd_output = std::process::Command::new("systemd-run")
        .args([
            "--user".to_string(),
            format!("--on-active={}", timer.duration().as_seconds_f32()),
            "--timer-property=AccuracySec=100ms".to_string(),
            std::env::current_exe()?.to_str().unwrap().to_string(),
            "timer".to_string(),
            "check".to_string(),
        ])
        .output()
        .with_context(|| "Failed to schedule systemd timer")?;

    io::stderr().write_all(&systemd_output.stderr)?;

    Ok(())
}

/// Stop the current long break timer.
pub fn stop(config: &Config) -> Result<()> {
    let status = Status::load(&config.state_file_path)?;

    if let Status::LongBreak(_) = status {
        Hook::LongBreakEnd.run(&config.hooks_directory)?;

        crate::clear(config)?;
    }

    Ok(())
}

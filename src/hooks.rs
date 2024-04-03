use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;

pub fn run_start_hook(hooks_directory: &Path) -> Result<()> {
    let start_hook_path = hooks_directory.join("start");

    if start_hook_path.exists() {
        println!(
            "Executing start hook at {}",
            start_hook_path.display().to_string().cyan()
        );

        std::process::Command::new(start_hook_path)
            .output()
            .with_context(|| "Failed to execute start hook")?;
    }

    Ok(())
}

pub fn run_stop_hook(hooks_directory: &Path) -> Result<()> {
    let stop_hook_path = hooks_directory.join("stop");

    if stop_hook_path.exists() {
        println!(
            "Executing stop hook at {}",
            stop_hook_path.display().to_string().cyan()
        );

        std::process::Command::new(stop_hook_path)
            .output()
            .with_context(|| "Failed to execute stop hook")?;
    }

    Ok(())
}

pub fn run_break_hook(hooks_directory: &Path) -> Result<()> {
    let break_hook_path = hooks_directory.join("break");

    if break_hook_path.exists() {
        println!(
            "Executing break hook at {}",
            break_hook_path.display().to_string().cyan()
        );

        std::process::Command::new(break_hook_path)
            .output()
            .with_context(|| "Failed to execute break hook")?;
    }

    Ok(())
}

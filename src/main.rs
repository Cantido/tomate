use std::path::PathBuf;

use anyhow::{anyhow, bail, Result};
use chrono::{prelude::*, TimeDelta};
use clap::{Parser, Subcommand};
use colored::Colorize;

use tomate::{config::{self, Config}, Pomodoro, Status};
use tomate::history::History;
use tomate::hooks;
use tomate::time::{TimeDeltaExt, Timer};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
    /// Config file to use. [default: ${XDG_CONFIG_DIR}/tomate/config.toml]
    config: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Get the current Pomodoro
    Status {
        /// Show a progress bar and don't exit until the current timer is over
        #[arg(short, long, default_value_t = false)]
        progress: bool,
        /// Print a custom-formatted status for the current Pomodoro
        ///
        /// Recognizes the following tokens:
        ///
        /// %d - description
        ///
        /// %t - tags, comma-separated
        ///
        /// %r - remaining time, in mm:ss format (or hh:mm:ss if longer than an hour)
        ///
        /// %R - remaining time in seconds
        ///
        /// %s - start time in RFC 3339 format
        ///
        /// %S - start time as a Unix timestamp
        ///
        /// %e - end time in RFC 3339 format
        ///
        /// %E - end time as a Unix timestamp
        #[arg(short, long)]
        format: Option<String>,
    },
    /// Start a Pomodoro
    Start {
        /// Length of the Pomodoro to start
        #[arg(short, long, value_parser = TimeDelta::from_human)]
        duration: Option<TimeDelta>,
        /// Description of the task you're focusing on
        description: Option<String>,
        /// Tags to categorize the work you're doing, comma-separated
        #[arg(short, long)]
        tags: Option<String>,
        /// Show a progress bar and don't exit until the current timer is over
        #[arg(short, long, default_value_t = false)]
        progress: bool,
    },
    /// Remove the existing Pomodoro, if any
    Clear,
    /// Finish a Pomodoro
    Finish,
    /// Take a break
    Break {
        /// Length of the break to start
        #[arg(short, long, value_parser = TimeDelta::from_human)]
        duration: Option<TimeDelta>,
        /// Show a progress bar and don't exit until the current timer is over
        #[arg(short, long, default_value_t = false)]
        progress: bool,
    },
    /// Print a list of all logged Pomodoros
    History,
    /// Delete all state and configuration files
    Purge,
}


fn load_state(state_path: &PathBuf) -> Result<Status> {
    if let Ok(true) = state_path.try_exists() {
        Ok(Status::load(state_path)?)
    } else {
        Ok(Status::Inactive)
    }
}

fn print_status(config: &Config, format: Option<String>, progress: bool) -> Result<()> {
    let status = load_state(&config.state_file_path)?;

    match status {
        Status::Active(pom) => {
            if let Some(format) = format {
                println!("{}", pom.format(&format, Local::now()));

                return Ok(());
            }

            if let Some(desc) = pom.description() {
                println!("Current Pomodoro: {}", desc.yellow());
            } else {
                println!("Current Pomodoro");
            }

            if pom.timer().done(Local::now()) {
                println!("Status: {}", "Done".red().bold());
            } else {
                println!("Status: {}", "Active".magenta().bold());
            }
            println!("Duration: {}", &pom.timer().duration().to_human().cyan());
            if let Some(tags) = pom.tags() {
                println!("Tags:");
                for tag in tags {
                    println!("\t- {}", tag.blue());
                }
            }
            println!();

            if progress {
                print_progress_bar(&pom.timer());
                println!();
                println!();
            } else {
                let remaining = pom.timer().remaining(Local::now());
                println!(
                    "Time remaining: {}",
                    &remaining.max(TimeDelta::zero()).to_kitchen()
                );
                println!();
            }
            println!(
                "{}",
                "(use \"tomate finish\" to archive this Pomodoro)".dimmed()
            );
            println!(
                "{}",
                "(use \"tomate clear\" to delete this Pomodoro)".dimmed()
            );
        }
        Status::Inactive => {
            println!("No current Pomodoro");
            println!();
            println!("{}", "(use \"tomate start\" to start a Pomodoro)".dimmed());
            println!("{}", "(use \"tomate break\" to take a break)".dimmed());
        }
        Status::ShortBreak(timer) => {
            println!("Taking a break");
            println!();

            if progress {
                print_progress_bar(&timer);
                println!();
                println!();
            } else {
                let remaining = timer.remaining(Local::now());
                println!(
                    "Time remaining: {}",
                    &remaining.max(TimeDelta::zero()).to_kitchen()
                );
                println!();
            }

            println!(
                "{}",
                "(use \"tomate finish\" to finish this break)".dimmed()
            );
        }
    }

    Ok(())
}

fn print_progress_bar(pom: &Timer) {
    let now = Local::now();
    let elapsed_ratio =
        pom.elapsed(now).num_milliseconds() as f32 / pom.duration().num_milliseconds() as f32;

    let bar_width = 40.0;

    let filled_count = (bar_width * elapsed_ratio).round() as usize;
    let unfilled_count = (bar_width * (1.0 - elapsed_ratio)).round() as usize;

    let filled_bar = vec!["█"; filled_count].join("");
    let unfilled_bar = vec!["░"; unfilled_count].join("");

    println!(
        "{} {}{} {}",
        &pom.elapsed(now).to_kitchen(),
        filled_bar,
        unfilled_bar,
        &pom.remaining(now).to_kitchen()
    );
}

fn start(config: &Config, pomodoro: Pomodoro, progress: bool) -> Result<()> {
    let status = load_state(&config.state_file_path)?;

    match status {
        Status::ShortBreak(_timer) => Err(anyhow!("You're currently taking a break!")),
        Status::Active(_pom) => Err(anyhow!("There is already an unfinished Pomodoro")),
        Status::Inactive => {
            let next_status = Status::Active(pomodoro);
            next_status.save(&config.state_file_path)?;

            hooks::run_start_hook(&config.hooks_directory)?;

            let timer = next_status.timer();

            if progress && timer.is_some() {
                println!();
                print_progress_bar(&timer.unwrap());
            }

            Ok(())
        }
    }
}

fn finish(config: &Config) -> Result<()> {
    let status = load_state(&config.state_file_path)?;

    match status {
        Status::Inactive => bail!("No active Pomodoro. Start one with \"tomate start\""),
        Status::ShortBreak(_timer) => {
            clear(config)?;
        }
        Status::Active(pom) => {
            History::append(&pom, &config.history_file_path)?;

            clear(config)?;
        }
    }

    Ok(())
}

fn clear(config: &Config) -> Result<()> {
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

fn take_break(config: &Config, timer: Timer, show_progress: bool) -> Result<()> {
    let status = load_state(&config.state_file_path)?;

    if matches!(status, Status::ShortBreak(_)) {
        bail!("You are already taking a break");
    }

    if !matches!(status, Status::Inactive) {
        bail!("Finish your current timer before taking a break");
    }

    let new_status = Status::ShortBreak(timer.clone());
    new_status.save(&config.state_file_path)?;

    hooks::run_break_hook(&config.hooks_directory)?;

    if show_progress {
        println!();
        print_progress_bar(&timer);
    }

    Ok(())
}

fn print_history(config: &Config) -> Result<()> {
    if !config.history_file_path.exists() {
        return Ok(());
    }

    let history = History::load(&config.history_file_path)?;

    history.print_std();

    Ok(())
}

fn purge(config: &Config) -> Result<()> {
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

fn main() -> Result<()> {
    let args = Args::parse();

    let config_path = if let Some(conf_path) = args.config {
        conf_path
    } else {
        config::default_config_path()?
    };

    let config = Config::init(&config_path)?;

    match &args.command {
        Command::Status { progress, format } => {
            print_status(&config, format.clone(), *progress)?;
        }
        Command::Start {
            duration,
            description,
            tags,
            progress,
        } => {
            let dur = duration.unwrap_or(config.pomodoro_duration);

            let mut pom = Pomodoro::new(Local::now(), dur);
            if let Some(desc) = description {
                pom.set_description(desc);
            }

            if let Some(tags) = tags {
                let tags: Vec<String> = tags.split(",").map(|s| s.to_string()).collect();

                pom.set_tags(tags);
            }

            start(&config, pom, *progress)?;
        }
        Command::Finish => {
            finish(&config)?;
        }
        Command::Clear => {
            clear(&config)?;
        }
        Command::Break { duration, progress } => {
            let dur = duration.unwrap_or(config.short_break_duration);

            let timer = Timer::new(Local::now(), dur);
            take_break(&config, timer, *progress)?;
        }
        Command::History => {
            print_history(&config)?;
        }
        Command::Purge => {
            purge(&config)?;

            if config_path.exists() {
                println!(
                    "Removing config file at {}",
                    config_path.display().to_string().cyan()
                );
                std::fs::remove_file(&config_path)?;
            }
        }
    }

    Ok(())
}

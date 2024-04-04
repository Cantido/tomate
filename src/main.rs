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


struct Program {
    pub config: Config,
    status: Status,
}

impl Program {
    fn new(config: Config) -> Self {
        Self {
            config,
            status: Status::Inactive,
        }
    }

    fn load_state(&mut self) -> Result<()> {
        let state_file_path = &self.config.state_file_path;

        self.status = if let Ok(true) = state_file_path.try_exists() {
            Status::load(state_file_path)?
        } else {
            Status::Inactive
        };

        Ok(())
    }

    fn print_status(&self, format: Option<String>, progress: bool) {
        match &self.status {
            Status::Active(pom) => {
                if let Some(format) = format {
                    println!("{}", pom.format(&format, Local::now()));

                    return;
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
                    Self::print_progress_bar(&pom.timer());
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
                    Self::print_progress_bar(&timer);
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

    fn start(&mut self, pomodoro: Pomodoro, progress: bool) -> Result<()> {
        match &self.status {
            Status::ShortBreak(_timer) => Err(anyhow!("You're currently taking a break!")),
            Status::Active(_pom) => Err(anyhow!("There is already an unfinished Pomodoro")),
            Status::Inactive => {
                self.status = Status::Active(pomodoro);
                self.status.save(&self.config.state_file_path)?;

                hooks::run_start_hook(&self.config.hooks_directory)?;

                let timer = self.status.timer();

                if progress && timer.is_some() {
                    println!();
                    Self::print_progress_bar(&timer.unwrap());
                }

                Ok(())
            }
        }
    }

    fn finish(&mut self) -> Result<()> {
        match &self.status {
            Status::Inactive => bail!("No active Pomodoro. Start one with \"tomate start\""),
            Status::ShortBreak(_timer) => {
                self.clear()?;
            }
            Status::Active(pom) => {
                History::append(&pom, &self.config.history_file_path)?;

                self.clear()?;
            }
        }

        Ok(())
    }

    fn clear(&mut self) -> Result<()> {
        let state_file_path = &self.config.state_file_path;

        if state_file_path.exists() {
            println!(
                "Deleting current Pomodoro state file {}",
                &self.config.state_file_path.display().to_string().cyan()
            );
            std::fs::remove_file(&self.config.state_file_path)?;
            self.status = Status::Inactive;

            hooks::run_stop_hook(&self.config.hooks_directory)?;
        }

        Ok(())
    }

    fn take_break(&mut self, timer: Timer, show_progress: bool) -> Result<()> {
        if matches!(self.status, Status::ShortBreak(_)) {
            bail!("You are already taking a break");
        }

        if !matches!(self.status, Status::Inactive) {
            bail!("Finish your current timer before taking a break");
        }

        self.status = Status::ShortBreak(timer.clone());
        self.status.save(&self.config.state_file_path)?;

        hooks::run_break_hook(&self.config.hooks_directory)?;

        if show_progress {
            println!();
            Self::print_progress_bar(&timer);
        }

        Ok(())
    }

    fn print_history(&self) -> Result<()> {
        if !self.config.history_file_path.exists() {
            return Ok(());
        }

        let history = History::load(&self.config.history_file_path)?;

        history.print_std();

        Ok(())
    }

    fn purge(&mut self) -> Result<()> {
        if self.config.state_file_path.exists() {
            println!(
                "Removing current Pomodoro file at {}",
                self.config.state_file_path.display().to_string().cyan()
            );
            std::fs::remove_file(&self.config.state_file_path)?;
        }

        if self.config.history_file_path.exists() {
            println!(
                "Removing history file at {}",
                self.config.history_file_path.display().to_string().cyan()
            );
            std::fs::remove_file(&self.config.history_file_path)?;
        }

        Ok(())
    }
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
            let mut state = Program::new(config);
            state.load_state()?;

            state.print_status(format.clone(), *progress);
        }
        Command::Start {
            duration,
            description,
            tags,
            progress,
        } => {
            let mut state = Program::new(config);
            state.load_state()?;

            let dur = duration.unwrap_or(state.config.pomodoro_duration);

            let mut pom = Pomodoro::new(Local::now(), dur);
            if let Some(desc) = description {
                pom.set_description(desc);
            }

            if let Some(tags) = tags {
                let tags: Vec<String> = tags.split(",").map(|s| s.to_string()).collect();

                pom.set_tags(tags);
            }

            state.start(pom, *progress)?;
        }
        Command::Finish => {
            let mut state = Program::new(config);
            state.load_state()?;

            state.finish()?;
        }
        Command::Clear => {
            let mut state = Program::new(config);
            state.load_state()?;

            state.clear()?;
        }
        Command::Break { duration, progress } => {
            let mut state = Program::new(config);
            state.load_state()?;

            let dur = duration.unwrap_or(state.config.short_break_duration);

            let timer = Timer::new(Local::now(), dur);
            state.take_break(timer, *progress)?;
        }
        Command::History => {
            let state = Program::new(config);

            state.print_history()?;
        }
        Command::Purge => {
            let mut state = Program::new(config);

            state.purge()?;

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

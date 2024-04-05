use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use chrono::prelude::*;
use clap::{Parser, Subcommand};
use colored::Colorize;
use prettytable::{color, format, Attr, Cell, Row, Table};

use regex::Regex;
use tomate::{Config, History, Pomodoro, Status};
use tomate::time::Timer;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Command,
    /// Config file to use. [default: ${XDG_CONFIG_DIR}/tomate/config.toml]
    config: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Get the current Pomodoro
    Status {
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
        #[arg(short, long, value_parser = duration_from_human)]
        duration: Option<Duration>,
        /// Description of the task you're focusing on
        description: Option<String>,
        /// Tags to categorize the work you're doing, comma-separated
        #[arg(short, long)]
        tags: Option<String>,
    },
    /// Remove the existing Pomodoro, if any
    Clear,
    /// Finish a Pomodoro
    Finish,
    /// Take a break
    Break {
        /// Length of the break to start
        #[arg(short, long, value_parser = duration_from_human)]
        duration: Option<Duration>,
        /// Take a long break instead of a short break
        #[arg(short, long, default_value_t = false)]
        long: bool,
    },
    /// Print a list of all logged Pomodoros
    History,
    /// Delete all state and configuration files
    Purge,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let config_path = if let Some(conf_path) = args.config {
        conf_path
    } else {
        tomate::default_config_path()?
    };

    let config = Config::init(&config_path)?;

    match &args.command {
        Command::Status { format } => {
            print_status(&config, format.clone())?;
        }
        Command::Start {
            duration,
            description,
            tags,
        } => {
            let dur = duration.unwrap_or(config.pomodoro_duration);

            let mut pom = Pomodoro::new(SystemTime::now(), dur);
            if let Some(desc) = description {
                pom.set_description(desc);
            }

            if let Some(tags) = tags {
                let tags: Vec<String> = tags.split(",").map(|s| s.to_string()).collect();

                pom.set_tags(tags);
            }

            tomate::start(&config, pom)?;

            println!();

            print_status(&config, None)?;

        }
        Command::Finish => {
            tomate::finish(&config)?;
        }
        Command::Clear => {
            tomate::clear(&config)?;
        }
        Command::Break { duration, long } => {

            let timer = if *long {
                let dur = duration.unwrap_or(config.long_break_duration);
                let timer = Timer::new(SystemTime::now(), dur);

                tomate::take_long_break(&config, timer.clone())?;
                timer
            } else {
                let dur = duration.unwrap_or(config.short_break_duration);
                let timer = Timer::new(SystemTime::now(), dur);

                tomate::take_short_break(&config, timer.clone())?;

                timer
            };

            println!();
            print_progress_bar(&timer);

        }
        Command::History => {
            if !config.history_file_path.exists() {
                return Ok(());
            }

            let history = History::load(&config.history_file_path)?;

            let mut table = Table::new();

            table.set_titles(Row::new(vec![
                Cell::new("Date Started").with_style(Attr::Underline(true)),
                Cell::new("Duration").with_style(Attr::Underline(true)),
                Cell::new("Tags").with_style(Attr::Underline(true)),
                Cell::new("Description").with_style(Attr::Underline(true)),
            ]));

            for pom in history.pomodoros().iter() {
                let starts_at: DateTime<Local> = pom.timer().starts_at().into();
                let date = starts_at.format("%d %b %R").to_string();
                let dur = tomate::time::duration::to_human(&pom.timer().duration());
                let tags = pom.tags().clone().unwrap_or(&["-".to_string()]).join(",");
                let desc = pom.description().clone().unwrap_or("-");

                table.add_row(Row::new(vec![
                    Cell::new(&date).with_style(Attr::ForegroundColor(color::BLUE)),
                    Cell::new(&dur)
                        .style_spec("r")
                        .with_style(Attr::ForegroundColor(color::CYAN)),
                    Cell::new(&tags),
                    Cell::new(&desc),
                ]));
            }
            table.set_format(*format::consts::FORMAT_CLEAN);
            table.printstd();
        }
        Command::Purge => {
            tomate::purge(&config)?;

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

fn print_status(config: &Config, format: Option<String>) -> Result<()> {
    let status = Status::load(&config.state_file_path)?;

    match status {
        Status::Active(pom) => {
            if let Some(format) = format {
                println!("{}", pom.format(&format, SystemTime::now()));

                return Ok(());
            }

            if let Some(desc) = pom.description() {
                println!("Current Pomodoro: {}", desc.yellow());
            } else {
                println!("Current Pomodoro");
            }

            if pom.timer().done(SystemTime::now()) {
                println!("Status: {}", "Done".red().bold());
            } else {
                println!("Status: {}", "Active".magenta().bold());
            }
            println!("Duration: {}", tomate::time::duration::to_human(&pom.timer().duration()).cyan());
            if let Some(tags) = pom.tags() {
                println!("Tags:");
                for tag in tags {
                    println!("\t- {}", tag.blue());
                }
            }
            println!();

            print_progress_bar(&pom.timer());
            println!();
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
            println!("Taking a short break");
            println!();

            print_progress_bar(&timer);
            println!();

            println!(
                "{}",
                "(use \"tomate finish\" to finish this break)".dimmed()
            );
        },
        Status::LongBreak(timer) => {
            println!("Taking a long break");
            println!();

            print_progress_bar(&timer);
            println!();

            println!(
                "{}",
                "(use \"tomate finish\" to finish this break)".dimmed()
            );
        },
    }

    Ok(())
}

fn duration_from_human(input: &str) -> Result<Duration> {
    let re = Regex::new(r"^(?:([0-9])h)?(?:([0-9]+)m)?(?:([0-9]+)s)?$").unwrap();
    let caps = re.captures(&input)
    .with_context(|| "Failed to parse duration string, format is <HOURS>h<MINUTES>m<SECONDS>s (each section is optional) example: 22m30s")?;

    let hours: u64 = caps.get(1).map_or("0", |c| c.as_str()).parse()?;
    let minutes: u64 = caps.get(2).map_or("0", |c| c.as_str()).parse()?;
    let seconds: u64 = caps.get(3).map_or("0", |c| c.as_str()).parse()?;

    let total_seconds = (hours * 3600) + (minutes * 60) + seconds;

    Ok(Duration::new(total_seconds, 0))
}

fn print_progress_bar(pom: &Timer) {
    let now = SystemTime::now();
    let elapsed_ratio =
        pom.elapsed(now).as_millis() as f32 / pom.duration().as_millis() as f32;

    let bar_width = 40.0;

    let filled_count = (bar_width * elapsed_ratio).round() as usize;
    let unfilled_count = (bar_width * (1.0 - elapsed_ratio)).round() as usize;

    let filled_bar = vec!["█"; filled_count].join("");
    let unfilled_bar = vec!["░"; unfilled_count].join("");

    println!(
        "{} {}{} {}",
        tomate::time::duration::to_kitchen(&pom.elapsed(now)),
        filled_bar,
        unfilled_bar,
        tomate::time::duration::to_kitchen(&pom.remaining(now)),
    );
}


use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{prelude::*, TimeDelta};
use clap::{Parser, Subcommand};
use colored::Colorize;
use human_panic::setup_panic;
use prettytable::{color, format, Attr, Cell, Row, Table};

use regex::Regex;
use tomate::{Config, History, Pomodoro, Status, Timer};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
    /// Config file to use. [default: ${XDG_CONFIG_DIR}/tomate/config.toml]
    #[arg(short, long)]
    config: Option<PathBuf>,
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
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
    /// Interact with Pomodoro timers
    #[command(visible_alias("pom"))]
    Pomodoro {
        #[command(subcommand)]
        command: PomodoroCommand,
    },
    /// Interact with short break timers
    #[command(visible_alias("short"))]
    ShortBreak {
        #[command(subcommand)]
        command: ShortBreakCommand,
    },
    /// Interact with long break timers
    #[command(visible_alias("long"))]
    LongBreak {
        #[command(subcommand)]
        command: LongBreakCommand,
    },
    /// Interact with system timers
    Timer {
        #[command(subcommand)]
        command: TimerCommand,
    },
    /// Print a list of all logged Pomodoros
    History {
        /// Print history data in JSON format
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Delete all state and configuration files
    Purge,
}

/// Commands for the current Pomodoro timer
#[derive(Debug, Subcommand)]
enum PomodoroCommand {
    /// Show the Pomodoro timer that is currently running
    Show {
        /// Customize how the current Pomodoro is printed.
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
    /// Start a new Pomodoro timer
    Start {
        /// Length of the Pomodoro to start like 2m30s
        #[arg(short, long, value_parser = duration_from_human)]
        duration: Option<TimeDelta>,
        /// Description of the task you're focusing on
        description: Option<String>,
        /// Tag to categorize the work you're doing. Can be provided multiple times, one for each
        /// tag
        #[arg(short, long)]
        tag: Vec<String>,
    },
    /// Stop the current Pomodoro timer and log it to the history file
    Stop,
}

// Commands for working with short breaks
#[derive(Debug, Subcommand)]
enum ShortBreakCommand {
    Start {
        /// Length of the short break to start like 2m30s
        #[arg(short, long, value_parser = duration_from_human)]
        duration: Option<TimeDelta>,
    },
    Stop,
}

// Commands for working with long breaks
#[derive(Debug, Subcommand)]
enum LongBreakCommand {
    Start {
        /// Length of the long break to start like 2m30s
        #[arg(short, long, value_parser = duration_from_human)]
        duration: Option<TimeDelta>,
    },
    Stop,
}

#[derive(Debug, Subcommand)]
enum TimerCommand {
    /// Check and execute any completed timers
    Check,
}

fn main() -> Result<()> {
    setup_panic!();
    env_logger::builder().format_timestamp(None).init();

    let args = Args::parse();

    let config_path = if let Some(conf_path) = args.config {
        conf_path
    } else {
        tomate::default_config_path().with_context(|| "Unable to find default config path")?
    };

    let config = Config::init(&config_path).with_context(|| "Failed to initialize config file")?;

    match &args.command {
        Command::Status { format } => {
            print_status(&config, format.clone())?;
        }
        Command::Pomodoro { command } => match &command {
            PomodoroCommand::Show { format } => print_status(&config, format.clone())?,
            PomodoroCommand::Start {
                duration,
                description,
                tag,
            } => {
                tomate::pomodoro::start(&config, duration, description, tag)?;
                print_status(&config, None)?;
            }
            PomodoroCommand::Stop => {
                tomate::pomodoro::stop(&config)?;
            }
        },
        Command::ShortBreak { command } => match &command {
            ShortBreakCommand::Start { duration } => {
                tomate::short_break::start(&config, duration)?;
                print_status(&config, None)?;
            }
            ShortBreakCommand::Stop => tomate::short_break::stop(&config)?,
        },
        Command::LongBreak { command } => match &command {
            LongBreakCommand::Start { duration } => {
                tomate::long_break::start(&config, duration)?;
                print_status(&config, None)?;
            }
            LongBreakCommand::Stop => tomate::long_break::stop(&config)?,
        },
        Command::Timer { command } => match command {
            TimerCommand::Check => {
                let status = Status::load(&config.state_file_path)?;

                match status {
                    Status::Active(pom) => {
                        if pom.timer().done(Local::now()) {
                            tomate::finish(&config)?;
                        }
                    }
                    Status::ShortBreak(timer) => {
                        if timer.done(Local::now()) {
                            tomate::finish(&config)?;
                        }
                    }
                    Status::LongBreak(timer) => {
                        if timer.done(Local::now()) {
                            tomate::finish(&config)?;
                        }
                    }
                    Status::Inactive => {
                        println!("No timers active");
                    }
                }
            }
        },
        Command::History { json } => {
            if !config.history_file_path.exists() {
                return Ok(());
            }

            let history = History::load(&config.history_file_path)?;

            let mut table = Table::new();

            if *json {
                let json_str = serde_json::to_string(&history)?;

                println!("{}", &json_str);
            } else {
                table.set_titles(Row::new(vec![
                    Cell::new("Date Started").with_style(Attr::Underline(true)),
                    Cell::new("Duration").with_style(Attr::Underline(true)),
                    Cell::new("Tags").with_style(Attr::Underline(true)),
                    Cell::new("Description").with_style(Attr::Underline(true)),
                ]));

                for pom in history.pomodoros().iter() {
                    let date = pom.timer().starts_at().format("%d %b %R").to_string();
                    let dur = to_human(&pom.timer().duration());
                    let tags = pom.tags().unwrap_or(&vec!["-".to_string()]).join(",");
                    let desc = pom.description().unwrap_or("-");

                    table.add_row(Row::new(vec![
                        Cell::new(&date).with_style(Attr::ForegroundColor(color::BLUE)),
                        Cell::new(&dur)
                            .style_spec("r")
                            .with_style(Attr::ForegroundColor(color::CYAN)),
                        Cell::new(&tags),
                        Cell::new(desc),
                    ]));
                }
                table.set_format(*format::consts::FORMAT_CLEAN);
                table.printstd();
            }
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

    if let Some(format) = format {
        match status {
            Status::Active(pom) => {
                println!("{}", format_pomodoro(&pom, &format, Local::now()));
            }
            Status::ShortBreak(timer) | Status::LongBreak(timer) => {
                println!("{}", format_timer(&timer, &format, Local::now()));
            }
            Status::Inactive => {
                // nothing!
            }
        }

        return Ok(());
    }

    match status {
        Status::Active(pom) => {
            if let Some(format) = format {
                println!("{}", format_pomodoro(&pom, &format, Local::now()));

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
            println!("Duration: {}", to_human(&pom.timer().duration()).cyan());
            if let Some(tags) = pom.tags() {
                println!("Tags:");
                for tag in tags {
                    println!("\t- {}", tag.blue());
                }
            }
            println!();

            print_progress_bar(pom.timer());
            println!();
            println!(
                "{}",
                "(use \"tomate pomodoro stop\" to archive this Pomodoro)".dimmed()
            );
        }
        Status::Inactive => {
            println!("No current Pomodoro");
            println!();
            println!(
                "{}",
                "(use \"tomate pomodoro start\" to start a Pomodoro)".dimmed()
            );
            println!(
                "{}",
                "(use \"tomate short-break start\" to take a break)".dimmed()
            );
        }
        Status::ShortBreak(timer) => {
            println!("Taking a short break");
            println!();

            print_progress_bar(&timer);
            println!();

            println!(
                "{}",
                "(use \"tomate short-break stop\" to finish this break)".dimmed()
            );
        }
        Status::LongBreak(timer) => {
            println!("Taking a long break");
            println!();

            print_progress_bar(&timer);
            println!();

            println!(
                "{}",
                "(use \"tomate long-break stop\" to finish this break)".dimmed()
            );
        }
    }

    Ok(())
}

fn duration_from_human(input: &str) -> Result<TimeDelta> {
    let re = Regex::new(r"^(?:([0-9])h)?(?:([0-9]+)m)?(?:([0-9]+)s)?$").unwrap();
    let caps = re.captures(input)
    .with_context(|| "Failed to parse duration string, format is <HOURS>h<MINUTES>m<SECONDS>s (each section is optional) example: 22m30s")?;

    let hours: i64 = caps.get(1).map_or("0", |c| c.as_str()).parse()?;
    let minutes: i64 = caps.get(2).map_or("0", |c| c.as_str()).parse()?;
    let seconds: i64 = caps.get(3).map_or("0", |c| c.as_str()).parse()?;

    let total_seconds = (hours * 3600) + (minutes * 60) + seconds;

    Ok(TimeDelta::new(total_seconds, 0).expect("Expected duration to be nonzero."))
}

fn to_human(duration: &TimeDelta) -> String {
    use std::fmt::Write;

    if duration.is_zero() {
        return "0s".to_string();
    }

    let hours = duration.num_seconds() / 3600;
    let minutes = (duration.num_seconds() / 60) - (hours * 60);
    let seconds = duration.num_seconds() % 60;

    let mut acc = String::new();

    if hours > 0 {
        write!(acc, "{}h", hours).unwrap();
    }

    if minutes > 0 {
        write!(acc, "{}m", minutes).unwrap();
    }

    if seconds > 0 {
        write!(acc, "{}s", seconds).unwrap();
    }

    acc
}

pub fn to_kitchen(duration: &TimeDelta) -> String {
    let hours = duration.num_seconds() / 3600;
    let minutes = (duration.num_seconds() / 60) - (hours * 60);
    let seconds = duration.num_seconds() % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

fn format_pomodoro(pomodoro: &Pomodoro, f: &str, now: DateTime<Local>) -> String {
    let output = f
        .replace("%d", pomodoro.description().unwrap_or(""))
        .replace(
            "%t",
            &pomodoro.tags().unwrap_or(&Vec::<String>::new()).join(","),
        )
        .replace("%r", &to_kitchen(&pomodoro.timer().remaining(now)))
        .replace(
            "%R",
            &pomodoro.timer().remaining(now).num_seconds().to_string(),
        )
        .replace("%s", &pomodoro.timer().starts_at().to_rfc3339())
        .replace("%S", &pomodoro.timer().starts_at().timestamp().to_string())
        .replace("%e", &pomodoro.timer().ends_at().to_rfc3339())
        .replace("%E", &pomodoro.timer().ends_at().timestamp().to_string());

    output
}

fn format_timer(timer: &Timer, f: &str, now: DateTime<Local>) -> String {
    f.replace("%r", &to_kitchen(&timer.remaining(now)))
        .replace("%R", &timer.remaining(now).num_seconds().to_string())
        .replace("%s", &timer.starts_at().to_rfc3339())
        .replace("%S", &timer.starts_at().timestamp().to_string())
        .replace("%e", &timer.ends_at().to_rfc3339())
        .replace("%E", &timer.ends_at().timestamp().to_string())
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
        to_kitchen(&pom.elapsed(now)),
        filled_bar,
        unfilled_bar,
        to_kitchen(&pom.remaining(now)),
    );
}

#[cfg(test)]
mod test {
    use chrono::{prelude::*, TimeDelta};

    use crate::{format_pomodoro, Pomodoro};

    #[test]
    fn pomodoro_format_wallclock() {
        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        let pom = Pomodoro::new(dt, dur);

        let actual_format = format_pomodoro(&pom, "%r", dt);

        assert_eq!(actual_format, "25:00");
    }

    #[test]
    fn pomodoro_format_description() {
        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        let mut pom = Pomodoro::new(dt, dur);
        pom.set_description("hello :)");

        let actual_format = format_pomodoro(&pom, "%d", dt);

        assert_eq!(actual_format, "hello :)");
    }

    #[test]
    fn pomodoro_format_remaining() {
        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        let pom = Pomodoro::new(dt, dur);

        let actual_format = format_pomodoro(&pom, "%R", dt);

        assert_eq!(actual_format, "1500");
    }

    #[test]
    fn pomodoro_format_start_iso() {
        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        let pom = Pomodoro::new(dt, dur);

        let actual_format = format_pomodoro(&pom, "%s", dt);
        let expected_format = dt.to_rfc3339();

        assert_eq!(actual_format, expected_format);
    }

    #[test]
    fn pomodoro_format_start_timestamp() {
        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        let pom = Pomodoro::new(dt, dur);

        let actual_format = format_pomodoro(&pom, "%S", dt);

        assert_eq!(actual_format, "1711562400");
    }

    #[test]
    fn pomodoro_format_tags() {
        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        let mut pom = Pomodoro::new(dt, dur);
        pom.set_tags(vec!["a".to_string(), "b".to_string(), "c".to_string()]);

        let actual_format = format_pomodoro(&pom, "%t", dt);

        assert_eq!(actual_format, "a,b,c");
    }

    #[test]
    fn pomodoro_format_eta() {
        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        let pom = Pomodoro::new(dt, dur);

        let actual_format = format_pomodoro(&pom, "%e", dt);
        let expected_format = (dt + dur).to_rfc3339();

        assert_eq!(actual_format, expected_format);
    }

    #[test]
    fn pomodoro_format_eta_timestamp() {
        let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
        let dur = TimeDelta::new(25 * 60, 0).unwrap();

        let pom = Pomodoro::new(dt, dur);

        let actual_format = format_pomodoro(&pom, "%E", dt);

        assert_eq!(actual_format, "1711563900");
    }
}

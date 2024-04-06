use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{prelude::*, TimeDelta};
use clap::{Parser, Subcommand};
use colored::Colorize;
use prettytable::{color, format, Attr, Cell, Row, Table};

use regex::Regex;
use tomate::{Config, History, Pomodoro, Status, Timer};

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
        duration: Option<TimeDelta>,
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
        duration: Option<TimeDelta>,
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
        Command::Start {
            duration,
            description,
            tags,
        } => {
            let dur = duration.unwrap_or(config.pomodoro_duration);

            let mut pom = Pomodoro::new(Local::now(), dur);
            if let Some(desc) = description {
                pom.set_description(desc);
            }

            if let Some(tags) = tags {
                let tags: Vec<String> = tags.split(',').map(|s| s.to_string()).collect();

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
                let timer = Timer::new(Local::now(), dur);

                tomate::take_long_break(&config, timer.clone())?;
                timer
            } else {
                let dur = duration.unwrap_or(config.short_break_duration);
                let timer = Timer::new(Local::now(), dur);

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
                let date = pom.timer().starts_at().format("%d %b %R").to_string();
                let dur = to_human(&pom.timer().duration());
                let tags = pom.tags().unwrap_or(&["-".to_string()]).join(",");
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
        }
        Status::LongBreak(timer) => {
            println!("Taking a long break");
            println!();

            print_progress_bar(&timer);
            println!();

            println!(
                "{}",
                "(use \"tomate finish\" to finish this break)".dimmed()
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

    Ok(TimeDelta::new(total_seconds, 0).expect("Expected duration to be within valid range"))
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

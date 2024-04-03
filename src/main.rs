use std::{
  fs::{read_to_string, OpenOptions}, io::prelude::*, path::PathBuf
};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{prelude::*, TimeDelta};
use clap::{Parser, Subcommand};
use colored::Colorize;
use prettytable::{color, format, Attr, Cell, Row, Table};
use serde::{Deserialize, Serialize};

use tomate::time::{Timer, TimeDeltaExt};
use tomate::config::{self, Config};
use tomate::hooks;

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

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "status")]
enum Status {
  Inactive,
  Active(Pomodoro),
  ShortBreak(Timer),
}

impl Status {
  fn timer(&self) -> Option<Timer> {
    match self {
      Status::Inactive => None,
      Status::Active(pom) => Some(pom.timer.clone()),
      Status::ShortBreak(timer) => Some(timer.clone()),
    }
  }
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

    self.status =
      if let Ok(true) = state_file_path.try_exists() {
        let state_str = read_to_string(state_file_path)?;
        let status: Status = toml::from_str(&state_str)?;

        status
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

          return
        }

        if let Some(desc) = &pom.description {
          println!("Current Pomodoro: {}", desc.yellow());
        } else {
          println!("Current Pomodoro");
        }

        if pom.timer.done(Local::now()) {
          println!("Status: {}", "Done".red().bold());
        } else {
          println!("Status: {}", "Active".magenta().bold());
        }
        println!("Duration: {}", &pom.timer.duration().to_human().cyan());
        if let Some(tags) = &pom.tags {
          println!("Tags:");
          for tag in tags {
            println!("\t- {}", tag.blue());
          }
        }
        println!();

        if progress {
          Self::print_progress_bar(&pom.timer);
          println!();
          println!();
        } else {
          let remaining = pom.timer.remaining(Local::now());
          println!("Time remaining: {}", &remaining.max(TimeDelta::zero()).to_kitchen());
          println!();
        }
        println!("{}", "(use \"tomate finish\" to archive this Pomodoro)".dimmed());
        println!("{}", "(use \"tomate clear\" to delete this Pomodoro)".dimmed());
      },
      Status::Inactive => {
        println!("No current Pomodoro");
        println!();
        println!("{}", "(use \"tomate start\" to start a Pomodoro)".dimmed());
        println!("{}", "(use \"tomate break\" to take a break)".dimmed());
      },
      Status::ShortBreak(timer) => {
        println!("Taking a break");
        println!();

        if progress {
          Self::print_progress_bar(&timer);
          println!();
          println!();
        } else {
          let remaining = timer.remaining(Local::now());
          println!("Time remaining: {}", &remaining.max(TimeDelta::zero()).to_kitchen());
          println!();
        }

        println!("{}", "(use \"tomate finish\" to finish this break)".dimmed());
      },
    }
  }

  fn print_progress_bar(pom: &Timer) {
    let now = Local::now();
    let elapsed_ratio = pom.elapsed(now).num_milliseconds() as f32 / pom.duration().num_milliseconds() as f32;

    let bar_width = 40.0;

    let filled_count = (bar_width * elapsed_ratio).round() as usize;
    let unfilled_count = (bar_width * (1.0 - elapsed_ratio)).round() as usize;

    let filled_bar = vec!["█"; filled_count].join("");
    let unfilled_bar = vec!["░"; unfilled_count].join("");


    println!("{} {}{} {}", &pom.elapsed(now).to_kitchen(), filled_bar, unfilled_bar, &pom.remaining(now).to_kitchen());
  }

  fn start(&mut self, pomodoro: Pomodoro, progress: bool) -> Result<()> {
    match &self.status {
      Status::ShortBreak(_timer) => Err(anyhow!("You're currently taking a break!")),
      Status::Active(_pom) => Err(anyhow!("There is already an unfinished Pomodoro")),
      Status::Inactive => {
        self.status = Status::Active(pomodoro);

        println!("Creating Pomodoro state file {}", &self.config.state_file_path.display().to_string().cyan());

        std::fs::create_dir_all(&self.config.state_file_path.parent().with_context(|| "State file path does not have a parent directory")?)?;
        std::fs::write(&self.config.state_file_path, toml::to_string(&self.status)?)?;

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
      },
      Status::Active(pom) => {
        let history_file_path = &self.config.history_file_path;
        let pom_str = toml::to_string(&pom)?;

        println!("Archiving Pomodoro to {}", &self.config.history_file_path.display().to_string().cyan());

        std::fs::create_dir_all(history_file_path.parent().with_context(|| "History file path does not have a parent directory")?)?;
        let mut history_file = OpenOptions::new().create(true).write(true).append(true).open(&history_file_path)?;
        writeln!(history_file, "[[pomodoros]]\n{}", pom_str)?;

        self.clear()?;
      }
    }

    Ok(())
  }

  fn clear(&mut self) -> Result<()> {
    let state_file_path = &self.config.state_file_path;

    if state_file_path.exists() {
      println!("Deleting current Pomodoro state file {}", &self.config.state_file_path.display().to_string().cyan());
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

    println!("Creating Pomodoro state file {}", &self.config.state_file_path.display().to_string().cyan());

    self.status = Status::ShortBreak(timer.clone());
    std::fs::create_dir_all(&self.config.state_file_path.parent().with_context(|| "State file path does not have a parent directory")?)?;
    std::fs::write(&self.config.state_file_path, toml::to_string(&self.status)?)?;

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

    let history_str = read_to_string(&self.config.history_file_path)?;
    let history: History = toml::from_str(&history_str)?;

    let mut table = Table::new();

    table.set_titles(Row::new(vec![
      Cell::new("Date Started")
          .with_style(Attr::Underline(true)),
      Cell::new("Duration")
          .with_style(Attr::Underline(true)),
      Cell::new("Tags")
          .with_style(Attr::Underline(true)),
      Cell::new("Description")
          .with_style(Attr::Underline(true)),
    ]));

    for pom in history.pomodoros.iter() {
      let date = pom.timer.starts_at().format("%d %b %R").to_string();
      let dur = &pom.timer.duration().to_human();
      let tags = pom.tags.clone().unwrap_or(vec!["-".to_string()]).join(",");
      let desc = pom.description.clone().unwrap_or("-".to_string());

      table.add_row(Row::new(vec![
        Cell::new(&date).with_style(Attr::ForegroundColor(color::BLUE)),
        Cell::new(&dur).style_spec("r").with_style(Attr::ForegroundColor(color::CYAN)),
        Cell::new(&tags),
        Cell::new(&desc),
      ]));
    }
    table.set_format(*format::consts::FORMAT_CLEAN);
    table.printstd();

    Ok(())
  }

  fn purge(&mut self) -> Result<()> {
    if self.config.state_file_path.exists() {
      println!("Removing current Pomodoro file at {}", self.config.state_file_path.display().to_string().cyan());
      std::fs::remove_file(&self.config.state_file_path)?;
    }

    if self.config.history_file_path.exists() {
      println!("Removing history file at {}", self.config.history_file_path.display().to_string().cyan());
      std::fs::remove_file(&self.config.history_file_path)?;
    }

    Ok(())
  }
}

#[derive(Debug, Serialize, Deserialize)]
struct Pomodoro {
  timer: Timer,
  description: Option<String>,
  tags: Option<Vec<String>>,
}

impl Pomodoro {
  fn new(starts_at: DateTime<Local>, duration: TimeDelta) -> Self {
    let timer = Timer::new(starts_at, duration);
    Self {
      timer,
      description: None,
      tags: None,
    }
  }

  pub fn set_description(&mut self, description: &str) {
    self.description = Some(description.to_string());
  }

  pub fn set_tags(&mut self, tags: Vec<String>) {
    self.tags = Some(tags);
  }

  fn format(&self, f: &str, now: DateTime<Local>) -> String {
    let output = f
      .replace("%d", &self.description.as_ref().unwrap_or(&"".to_string()))
      .replace("%t", &self.tags.as_ref().unwrap_or(&Vec::<String>::new()).join(","))
      .replace("%r", &self.timer.remaining(now).to_kitchen())
      .replace("%R", &self.timer.remaining(now).num_seconds().to_string())
      .replace("%s", &self.timer.starts_at().to_rfc3339())
      .replace("%S", &self.timer.starts_at().timestamp().to_string())
      .replace("%e", &self.timer.ends_at().to_rfc3339())
      .replace("%E", &self.timer.ends_at().timestamp().to_string());

    output
  }
}


#[derive(Debug, Deserialize, Serialize)]
struct History {
  pomodoros: Vec<Pomodoro>
}

fn main() -> Result<()> {
    let args = Args::parse();

    let config_path =
      if let Some(conf_path) = args.config {
        conf_path
      } else {
        config::default_config_path()?
      };

    let config =
      if let Some(conf) = Config::load(&config_path)? {
        conf
      } else {
        let conf = Config::default();

        println!("Creating config file at {}", config_path.display().to_string().cyan());
        println!();
        std::fs::write(&config_path, toml::to_string(&conf)?)?;

        conf
      };


    match &args.command {
      Command::Status { progress, format } => {
        let mut state = Program::new(config);
        state.load_state()?;

        state.print_status(format.clone(), *progress);
      },
      Command::Start{ duration, description, tags, progress } => {
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
      },
      Command::Finish => {
        let mut state = Program::new(config);
        state.load_state()?;

        state.finish()?;
      },
      Command::Clear => {
        let mut state = Program::new(config);
        state.load_state()?;

        state.clear()?;
      },
      Command::Break { duration, progress } => {
        let mut state = Program::new(config);
        state.load_state()?;

        let dur = duration.unwrap_or(state.config.short_break_duration);

        let timer = Timer::new(Local::now(), dur);
        state.take_break(timer, *progress)?;
      },
      Command::History => {
        let state = Program::new(config);

        state.print_history()?;
      },
      Command::Purge => {
        let mut state = Program::new(config);

        state.purge()?;

        if config_path.exists() {
          println!("Removing config file at {}", config_path.display().to_string().cyan());
          std::fs::remove_file(&config_path)?;
        }
      },
    }

    Ok(())
}


#[cfg(test)]
mod test {
    use chrono::{prelude::*, TimeDelta};

    use crate::{Pomodoro, Status};

  #[test]
  fn status_to_toml() {
    let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
    let dur = TimeDelta::new(25 * 60, 0).unwrap();

    let pom = Pomodoro::new(dt, dur);

    let status = Status::Active(pom);

    let toml = toml::to_string_pretty(&status).unwrap();
    let lines: Vec<&str> = toml.lines().collect();

    assert_eq!(lines[0], "status = \"Active\"");
    assert_eq!(lines[1], "timer = \"2024-03-27T12:00:00-06:00/PT1500S\"");
  }

  #[test]
  fn toml_to_pom() {
    let pom: Pomodoro = toml::from_str(r#"
      timer = "2024-03-27T12:00:00-06:00/PT1500S"
    "#).expect("Could not parse pomodoro from string");

    let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
    let dur = TimeDelta::new(25 * 60, 0).unwrap();

    assert_eq!(pom.timer.starts_at(), dt);
    assert_eq!(pom.timer.duration(), dur);
  }

  #[test]
  fn time_elapsed() {
    let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
    let dt_later: DateTime<Local> = "2024-03-27T12:20:00-06:00".parse().unwrap();
    let dur = TimeDelta::new(25 * 60, 0).unwrap();

    let pom = Pomodoro::new(dt, dur);

    let expected_elapsed = TimeDelta::new(20 * 60, 0).unwrap();

    assert_eq!(pom.timer.elapsed(dt_later), expected_elapsed);
  }


  #[test]
  fn time_remaining() {
    let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
    let dt_later: DateTime<Local> = "2024-03-27T12:20:00-06:00".parse().unwrap();
    let dur = TimeDelta::new(25 * 60, 0).unwrap();

    let pom = Pomodoro::new(dt, dur);

    let expected_remaining = TimeDelta::new(5 * 60, 0).unwrap();

    assert_eq!(pom.timer.remaining(dt_later), expected_remaining);
  }

  #[test]
  fn pomodoro_format_wallclock() {
    let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
    let dur = TimeDelta::new(25 * 60, 0).unwrap();

    let pom = Pomodoro::new(dt, dur);

    let actual_format = pom.format("%r", dt);

    assert_eq!(actual_format, "25:00");
  }

  #[test]
  fn pomodoro_format_description() {
    let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
    let dur = TimeDelta::new(25 * 60, 0).unwrap();

    let mut pom = Pomodoro::new(dt, dur);
    pom.set_description("hello :)");

    let actual_format = pom.format("%d", dt);

    assert_eq!(actual_format, "hello :)");
  }

  #[test]
  fn pomodoro_format_remaining() {
    let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
    let dur = TimeDelta::new(25 * 60, 0).unwrap();

    let pom = Pomodoro::new(dt, dur);

    let actual_format = pom.format("%R", dt);

    assert_eq!(actual_format, "1500");
  }

  #[test]
  fn pomodoro_format_start_iso() {
    let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
    let dur = TimeDelta::new(25 * 60, 0).unwrap();

    let pom = Pomodoro::new(dt, dur);

    let actual_format = pom.format("%s", dt);

    assert_eq!(actual_format, "2024-03-27T12:00:00-06:00");
  }

  #[test]
  fn pomodoro_format_start_timestamp() {
    let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
    let dur = TimeDelta::new(25 * 60, 0).unwrap();

    let pom = Pomodoro::new(dt, dur);

    let actual_format = pom.format("%S", dt);

    assert_eq!(actual_format, "1711562400");
  }

  #[test]
  fn pomodoro_format_tags() {
    let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
    let dur = TimeDelta::new(25 * 60, 0).unwrap();

    let mut pom = Pomodoro::new(dt, dur);
    pom.set_tags(vec!["a".to_string(), "b".to_string(), "c".to_string()]);

    let actual_format = pom.format("%t", dt);

    assert_eq!(actual_format, "a,b,c");
  }

  #[test]
  fn pomodoro_format_eta() {
    let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
    let dur = TimeDelta::new(25 * 60, 0).unwrap();

    let pom = Pomodoro::new(dt, dur);

    let actual_format = pom.format("%e", dt);

    assert_eq!(actual_format, "2024-03-27T12:25:00-06:00");
  }

  #[test]
  fn pomodoro_format_eta_timestamp() {
    let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
    let dur = TimeDelta::new(25 * 60, 0).unwrap();

    let pom = Pomodoro::new(dt, dur);

    let actual_format = pom.format("%E", dt);

    assert_eq!(actual_format, "1711563900");
  }
}

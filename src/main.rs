use std::{
  fs::{read_to_string, OpenOptions}, io::prelude::*, path::PathBuf
};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{prelude::*, TimeDelta};
use clap::{Parser, Subcommand};
use directories::ProjectDirs;
use prettytable::{format, Attr, Cell, Row, Table};
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
  #[command(subcommand)]
  command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
  /// Get the current Pomodoro
  Status,
  /// Start a Pomodoro
  Start {
    /// Length of the Pomodoro to start, in minutes
    #[arg(short, long, default_value_t = 25)]
    minutes: u64,
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
  /// Print a list of all logged Pomodoros
  History,
  /// Delete all state and configuration files
  Purge,
}

enum Status {
  Inactive,
  Active(TimeDelta),
  Done,
}

struct State {
  pub config: Config,
  pub current_pomodoro: Option<Pomodoro>,
}

impl State {
  fn load(config: Config) -> Result<Self> {
    let state_file_path = &config.state_file_path;

    let current_pomodoro = {
      if let Ok(true) = state_file_path.try_exists() {
        let state_str = read_to_string(state_file_path)?;
        let pom: Pomodoro = toml::from_str(&state_str)?;

        Some(pom)
      } else {
        None
      }
    };

    Ok(Self {
      config,
      current_pomodoro,
    })
  }

  fn status(&self) -> Status {
    match &self.current_pomodoro {
      Some(pom) => {
        let time_remaining = pom.time_remaining(Local::now());

        if time_remaining > TimeDelta::zero() {
          Status::Active(time_remaining)
        } else {
          Status::Done
        }
      },
      None => Status::Inactive
    }
  }

  fn print_status(&self) {
    match self.status() {
      Status::Done => {
        let pom = self.current_pomodoro.as_ref().unwrap();
        if let Some(desc) = &pom.description {
          println!("Pomodoro done: {}", desc);
        } else {
          println!("Pomodoro done");
        }
        println!("Duration: {}", human_duration(&pom.duration));
        if let Some(tags) = &pom.tags {
          println!("Tags:");
          for tag in tags {
            println!("\t- {}", tag);
          }
        }
        println!("");
        println!("(use \"tomate finish\" to archive this Pomodoro)");
        println!("(use \"tomate clear\" to delete this Pomodoro)");
      },
      Status::Active(time_remaining) => {
        let pom = self.current_pomodoro.as_ref().unwrap();
        if let Some(desc) = &pom.description {
          println!("Pomodoro active: {}", desc);
        } else {
          println!("Pomodoro active");
        }
        println!("Duration: {}", human_duration(&pom.duration));
        if let Some(tags) = &pom.tags {
          println!("Tags:");
          for tag in tags {
            println!("\t- {}", tag);
          }
        }
        println!("");
        println!("Time remaining: {}", wallclock(&time_remaining));
        println!("");
        println!("(use \"tomate finish\" to archive this Pomodoro)");
        println!("(use \"tomate clear\" to delete this Pomodoro)");
      },
      Status::Inactive => {
        println!("No active Pomodoro");
        println!("");
        println!("(use \"tomate start\" to start a Pomodoro)");
      },
    }
  }

  fn start(&mut self, pomodoro: Pomodoro) -> Result<()> {
    match &self.status() {
      Status::Done => Err(anyhow!("There is already an unfinished Pomodoro")),
      Status::Active(_time_remaining) => Err(anyhow!("There is already an unfinished Pomodoro")),
      Status::Inactive => {
        self.current_pomodoro = Some(pomodoro);

        println!("Creating Pomodoro state file {}", &self.config.state_file_path.display());

        std::fs::create_dir_all(&self.config.state_file_path.parent().with_context(|| "State file path does not have a parent directory")?)?;
        std::fs::write(&self.config.state_file_path, toml::to_string(&self.current_pomodoro)?)?;

        Ok(())
      }
    }
  }

  fn finish(&mut self) -> Result<()> {
    if matches!(&self.status(), Status::Inactive) {
      bail!("No active Pomodoro. Start one with \"tomate start\"");
    }

    let state_file_path = &self.config.state_file_path;
    let history_file_path = &self.config.history_file_path;
    let state_str = read_to_string(&state_file_path)?;

    println!("Archiving Pomodoro to {}", &self.config.history_file_path.display());

    std::fs::create_dir_all(history_file_path.parent().with_context(|| "History file path does not have a parent directory")?)?;
    let mut history_file = OpenOptions::new().create(true).write(true).append(true).open(&history_file_path)?;
    writeln!(history_file, "[[pomodoros]]\n{}", state_str)?;

    self.clear()?;

    Ok(())
  }

  fn clear(&mut self) -> Result<()> {
    let state_file_path = &self.config.state_file_path;

    if state_file_path.exists() {
      println!("Deleting current Pomodoro state file {}", &self.config.state_file_path.display());
      std::fs::remove_file(&self.config.state_file_path)?;
      self.current_pomodoro = None;
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
      let date = pom.started_at.format("%d %b %R").to_string();
      let dur = human_duration(&pom.duration);
      let tags = pom.tags.clone().unwrap_or(vec!["-".to_string()]).join(",");
      let desc = pom.description.clone().unwrap_or("-".to_string());

      table.add_row(Row::new(vec![
        Cell::new(&date),
        Cell::new(&dur).style_spec("r").with_style(Attr::Dim),
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
      println!("Removing current Pomodoro file at {}", self.config.state_file_path.display());
      std::fs::remove_file(&self.config.state_file_path)?;
    }

    if self.config.history_file_path.exists() {
      println!("Removing Tomate history file at {}", self.config.history_file_path.display());
      std::fs::remove_file(&self.config.history_file_path)?;
    }

    Ok(())
  }
}

#[derive(Debug, Serialize, Deserialize)]
struct Pomodoro {
  #[serde(rename = "start_time")]
  started_at: DateTime<Local>,
  #[serde(deserialize_with = "from_period_string", serialize_with = "to_period_string")]
  duration: TimeDelta,
  description: Option<String>,
  tags: Option<Vec<String>>,
}

impl Pomodoro {
  fn new(started_at: DateTime<Local>, duration: TimeDelta) -> Self {
    Self {
      started_at,
      duration,
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

  fn time_elapsed(&self, now: DateTime<Local>) -> TimeDelta {
    now - self.started_at
  }

  fn time_remaining(&self, now: DateTime<Local>) -> TimeDelta {
    self.duration - self.time_elapsed(now)
  }
}

fn from_period_string<'de, D>(deserializer: D) -> Result<TimeDelta, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let re = Regex::new(r"^PT([0-9]+)S$").unwrap();
    let cap = re.captures(&s)
      .with_context(|| "Failed to apply regex to duration string").unwrap()
      .get(1)
      .with_context(|| "String does not seem to be a duration string").unwrap()
      .as_str();
    let seconds: i64 = cap.parse()
      .with_context(|| format!("String {} is not an integer", cap)).unwrap();

    Ok(TimeDelta::new(seconds, 0).unwrap())
}

fn to_period_string<S>(delta: &TimeDelta, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer
{
  serializer.serialize_str(&delta.to_string())
}

#[derive(Debug, Deserialize, Serialize)]
struct History {
  pomodoros: Vec<Pomodoro>
}

struct Config {
  pub state_file_path: PathBuf,
  pub history_file_path: PathBuf,
}

impl Default for Config {
  fn default() -> Self {
      let project_dirs =
        ProjectDirs::from("dev", "Cosmicrose", "Tomate")
        .with_context(|| "Unable to determine XDG directories").unwrap();

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
        state_file_path,
        history_file_path,
      }
  }
}


fn main() -> Result<()> {
    let config = Config::default();

    let args = Args::parse();

    match &args.command {
      Command::Status => {
        let state = State::load(config)?;
        state.print_status();
      },
      Command::Start{ minutes, description, tags } => {
        let mut state = State::load(config)?;

        let minutes: i64 = minutes.clone().try_into().unwrap();
        let dur = TimeDelta::new(minutes * 60, 0).unwrap();

        let mut pom = Pomodoro::new(Local::now(), dur);

        if let Some(desc) = description {
          pom.set_description(desc);
        }

        if let Some(tags) = tags {
          let tags: Vec<String> = tags.split(",").map(|s| s.to_string()).collect();

          pom.set_tags(tags);
        }

        state.start(pom)?;
      },
      Command::Finish => {
        let mut state = State::load(config)?;

        state.finish()?;
      },
      Command::Clear => {
        let mut state = State::load(config)?;

        state.clear()?;
      },
      Command::History => {
        let state = State::load(config)?;

        state.print_history()?;
      },
      Command::Purge => {
        let mut state = State::load(config)?;

        state.purge()?;
      },
    }

    Ok(())
}

fn wallclock(t: &TimeDelta) -> String {
  let minutes = t.num_minutes();
  let seconds = t.num_seconds() - (minutes * 60);

  format!("{:02}:{:02}", minutes, seconds)
}

fn human_duration(t: &TimeDelta) -> String {
  use std::fmt::Write;

  let minutes = t.num_minutes();
  let seconds = t.num_seconds() - (minutes * 60);

  let mut acc = String::new();

  if minutes > 0 {
    write!(acc, "{}m", minutes).unwrap();
  }

  if seconds > 0 {
    write!(acc, "{}s", minutes).unwrap();
  }

  acc
}

#[cfg(test)]
mod test {
    use chrono::{prelude::*, TimeDelta};

    use crate::{Pomodoro, wallclock};

  #[test]
  fn pom_to_toml() {
    let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
    let dur = TimeDelta::new(25 * 60, 0).unwrap();

    let pom = Pomodoro::new(dt, dur);

    let toml = toml::to_string_pretty(&pom).unwrap();
    let lines: Vec<&str> = toml.lines().collect();

    assert_eq!(lines[0], "start_time = \"2024-03-27T12:00:00-06:00\"");
    assert_eq!(lines[1], "duration = \"PT1500S\"");
  }

  #[test]
  fn toml_to_pom() {
    let pom: Pomodoro = toml::from_str(r#"
      start_time = "2024-03-27T12:00:00-06:00"
      duration = "PT1500S"
    "#).expect("Could not parse pomodoro from string");

    let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
    let dur = TimeDelta::new(25 * 60, 0).unwrap();

    assert_eq!(pom.started_at, dt);
    assert_eq!(pom.duration, dur);
  }

  #[test]
  fn time_elapsed() {
    let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
    let dt_later: DateTime<Local> = "2024-03-27T12:20:00-06:00".parse().unwrap();
    let dur = TimeDelta::new(25 * 60, 0).unwrap();

    let pom = Pomodoro::new(dt, dur);

    let expected_elapsed = TimeDelta::new(20 * 60, 0).unwrap();

    assert_eq!(pom.time_elapsed(dt_later), expected_elapsed);
  }


  #[test]
  fn time_remaining() {
    let dt: DateTime<Local> = "2024-03-27T12:00:00-06:00".parse().unwrap();
    let dt_later: DateTime<Local> = "2024-03-27T12:20:00-06:00".parse().unwrap();
    let dur = TimeDelta::new(25 * 60, 0).unwrap();

    let pom = Pomodoro::new(dt, dur);

    let expected_remaining = TimeDelta::new(5 * 60, 0).unwrap();

    assert_eq!(pom.time_remaining(dt_later), expected_remaining);
  }

  #[test]
  fn wallclock_test() {
    let dur = TimeDelta::new(25 * 60, 0).unwrap();

    let clock = wallclock(&dur);

    assert_eq!(clock, "25:00");
  }

  #[test]
  fn wallclock_seconds_test() {
    let dur = TimeDelta::new(12, 0).unwrap();

    let clock = wallclock(&dur);

    assert_eq!(clock, "00:12");
  }
}

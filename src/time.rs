use anyhow::{Context, Result};
use chrono::{DateTime, Local, TimeDelta};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Extensions to `TimeDelta`
pub trait TimeDeltaExt
where
  Self: Sized
{
  /// Parse a `TimeDelta` from an ISO 8601 string, for example "PT1500S".
  fn from_iso8601(s: &str) -> Result<Self>;

  /// Formats the TimeDelta as a "kitchen timer" string, e.g. mm:ss.
  ///
  /// If the delta is longer than an hour, the delta is formatted as hh:mm:ss.
  fn to_kitchen(&self) -> String;

  /// Formats the TimeDelta in a humanized way, for example 22m30s.
  fn to_human(&self) -> String;
}

impl TimeDeltaExt for TimeDelta {
  fn from_iso8601(s: &str) -> Result<Self> {
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

  fn to_kitchen(&self) -> String {
    let hours = self.num_hours();
    let minutes = self.num_minutes() - (hours * 60);
    let seconds = self.num_seconds() - (minutes * 60);

    if hours > 0 {
      format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
      format!("{:02}:{:02}", minutes, seconds)
    }
  }

  fn to_human(&self) -> String {
    use std::fmt::Write;

    if self.is_zero() {
      return "0s".to_string();
    }

    let hours = self.num_hours();
    let minutes = self.num_minutes() - (hours * 60);
    let seconds = self.num_seconds() - (minutes * 60);

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
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Timer {
  started_at: DateTime<Local>,
  #[serde(with = "crate::duration")]
  duration: TimeDelta,
}

impl Timer {
  pub fn new(started_at: DateTime<Local>, duration: TimeDelta) -> Self {
    Self {
      started_at,
      duration,
    }
  }

  pub fn starts_at(&self) -> DateTime<Local> {
    self.started_at
  }

  pub fn ends_at(&self) -> DateTime<Local> {
    self.started_at + self.duration
  }

  pub fn duration(&self) -> TimeDelta {
    self.duration
  }

  pub fn time_elapsed(&self, now: DateTime<Local>) -> TimeDelta {
    now - self.started_at
  }

  pub fn time_remaining(&self, now: DateTime<Local>) -> TimeDelta {
    self.duration - self.time_elapsed(now)
  }
}

#[cfg(test)]
mod test {
    use chrono::TimeDelta;

    use crate::time::TimeDeltaExt;

  #[test]
  fn kitchen_test() {
    let dur = TimeDelta::new(25 * 60, 0).unwrap();

    let clock = &dur.to_kitchen();

    assert_eq!(clock, "25:00");
  }

  #[test]
  fn kitchen_seconds_test() {
    let dur = TimeDelta::new(12, 0).unwrap();

    let clock = &dur.to_kitchen();

    assert_eq!(clock, "00:12");
  }
}

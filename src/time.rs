use chrono::TimeDelta;

pub fn wallclock(t: &TimeDelta) -> String {
  let hours = t.num_hours();
  let minutes = t.num_minutes() - (hours * 60);
  let seconds = t.num_seconds() - (minutes * 60);

  if hours > 0 {
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
  } else {
    format!("{:02}:{:02}", minutes, seconds)
  }
}

pub fn human_duration(t: &TimeDelta) -> String {
  use std::fmt::Write;

  if t.is_zero() {
    return "0s".to_string();
  }

  let hours = t.num_hours();
  let minutes = t.num_minutes() - (hours * 60);
  let seconds = t.num_seconds() - (minutes * 60);

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

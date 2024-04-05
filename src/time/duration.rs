use std::time::Duration;


pub fn to_kitchen(duration: &Duration) -> String {
    let hours = duration.as_secs() / 3600;
    let minutes = (duration.as_secs() / 60) - (hours * 60);
    let seconds = duration.as_secs() % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

pub fn to_human(duration: &Duration) -> String {
    use std::fmt::Write;

    if duration.is_zero() {
        return "0s".to_string();
    }

    let hours = duration.as_secs() / 3600;
    let minutes = (duration.as_secs() / 60) - (hours * 60);
    let seconds = duration.as_secs() % 60;

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

pub mod seconds {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let sec: u64 = Deserialize::deserialize(deserializer)?;
        Ok(Duration::new(sec, 0))
    }

    pub fn serialize<S>(delta: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(delta.as_secs())
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::time::duration::to_kitchen;

    #[test]
    fn kitchen_test() {
        let dur = Duration::new(25 * 60, 0);

        let clock = to_kitchen(&dur);

        assert_eq!(clock, "25:00");
    }

    #[test]
    fn kitchen_seconds_test() {
        let dur = Duration::new(12, 0);

        let clock = to_kitchen(&dur);

        assert_eq!(clock, "00:12");
    }
}

use chrono::TimeDelta;
use serde::{Deserialize, Deserializer, Serializer};

use crate::time::TimeDeltaExt;

pub fn deserialize<'de, D>(deserializer: D) -> Result<TimeDelta, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    Ok(TimeDelta::from_iso8601(&s).unwrap())
}

pub fn serialize<S>(delta: &TimeDelta, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer
{
  serializer.serialize_str(&delta.to_string())
}

#[doc(hidden)]
pub mod seconds {
    use chrono::TimeDelta;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<TimeDelta, D::Error>
    where
        D: Deserializer<'de>,
    {
        let sec: i64 = Deserialize::deserialize(deserializer)?;
        Ok(TimeDelta::new(sec, 0).unwrap())
    }

    pub fn serialize<S>(delta: &TimeDelta, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(delta.num_seconds())
    }
}

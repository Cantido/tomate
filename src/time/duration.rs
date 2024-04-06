#[doc(hidden)]
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

pub mod unix {
    use std::time::{Duration, SystemTime};

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let ts: u64 = Deserialize::deserialize(deserializer)?;

        let time = SystemTime::UNIX_EPOCH + Duration::from_secs(ts);

        Ok(time)
    }

    pub fn serialize<S>(dt: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(dt.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs())
    }
}

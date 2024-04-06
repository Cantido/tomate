#[doc(hidden)]
pub mod unix {
    use chrono::prelude::*;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Local>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let ts: i64 = Deserialize::deserialize(deserializer)?;
        Ok(Local.timestamp_opt(ts, 0).unwrap())
    }

    pub fn serialize<S>(dt: &DateTime<Local>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(dt.timestamp())
    }
}

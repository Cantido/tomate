#[doc(hidden)]
pub mod unix {
    use chrono::prelude::*;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Local>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let ts: Option<i64> = Deserialize::deserialize(deserializer)?;

        match ts {
            Some(ts) => Ok(Some(Local.timestamp_opt(ts, 0).unwrap())),
            None => Ok(None),
        }
    }

    pub fn serialize<S>(dt: &Option<DateTime<Local>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match dt {
            Some(ref dt) => serializer.serialize_some(&dt.timestamp()),
            None => serializer.serialize_none(),
        }
    }
}

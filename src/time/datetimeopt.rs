#[doc(hidden)]
pub mod unix {
    use std::time::{Duration, SystemTime};

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SystemTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let ts: Option<u64> = Deserialize::deserialize(deserializer)?;

        match ts {
            Some(ts) => Ok(Some(SystemTime::UNIX_EPOCH + Duration::from_secs(ts))),
            None => Ok(None),
        }
    }

    pub fn serialize<S>(dt: &Option<SystemTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match dt {
            Some(ref dt) => serializer.serialize_some(&dt.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()),
            None => serializer.serialize_none(),
        }
    }
}

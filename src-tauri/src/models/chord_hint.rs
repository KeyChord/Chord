use crate::input::Key;
use regex::Regex;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordHint {
    #[typeshare(typescript(type = "{ keys: string[] } | { regex: string }"))]
    pub pattern: ChordHintPattern,
    pub raw_pattern: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub enum ChordHintPattern {
    Keys(Vec<Key>),
    Regex(Regex),
}

impl Serialize for ChordHintPattern {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ChordHintPattern::Keys(keys) => {
                let mut s = serializer.serialize_struct("ChordTrigger", 1)?;
                s.serialize_field("keys", keys)?;
                s.end()
            }
            ChordHintPattern::Regex(regex) => {
                let mut s = serializer.serialize_struct("ChordTrigger", 1)?;
                s.serialize_field("regex", regex.as_str())?;
                s.end()
            }
        }
    }
}

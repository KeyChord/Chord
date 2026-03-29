use bitflags::__private::serde::Serializer;
use regex::Regex;
use serde::ser::SerializeStruct;
use serde::Serialize;
use typeshare::typeshare;
use crate::input::Key;

#[typeshare]
#[derive(Debug, Serialize, Clone)]
pub struct ChordHint {
    #[typeshare(typescript(type = "any"))]
    pub pattern: ChordHintPattern,
    pub description: String
}

#[derive(Debug, Clone)]
pub enum ChordHintPattern {
    Keys(Vec<Key>),
    Regex(Regex)
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
                s.serialize_field("pattern", regex.as_str())?;
                s.end()
            }
        }
    }
}

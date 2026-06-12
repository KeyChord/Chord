use super::Key;
use regex::Regex;
use serde::ser::{Serialize, SerializeStruct, Serializer};

#[derive(Debug, Clone)]
pub enum ChordTrigger {
    Keys(Vec<Key>),
    Pattern(Regex),
}

impl ChordTrigger {
    pub fn matches(&self, keys: &[Key]) -> bool {
        match self {
            ChordTrigger::Keys(trigger_keys) => trigger_keys == keys,
            ChordTrigger::Pattern(regex) => {
                let Some(sequence_str) = Key::serialize_sequence(keys) else {
                    return false;
                };

                regex.is_match(&sequence_str)
            }
        }
    }
}

impl Serialize for ChordTrigger {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ChordTrigger::Keys(keys) => {
                let mut s = serializer.serialize_struct("ChordTrigger", 1)?;
                s.serialize_field("keys", keys)?;
                s.end()
            }
            ChordTrigger::Pattern(regex) => {
                let mut s = serializer.serialize_struct("ChordTrigger", 1)?;
                s.serialize_field("pattern", regex.as_str())?;
                s.end()
            }
        }
    }
}

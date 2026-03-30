use crate::input::Key;

/// A chord inputted by the user.
#[derive(Debug)]
pub struct ChordInput {
    pub keys: Vec<Key>,
    pub application_id: Option<String>,
}

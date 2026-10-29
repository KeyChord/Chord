use crate::define_observable;
use crate::models::{ChordInput, ChordInputEvent, typed_quote_count};
use serde::Serialize;
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChordInputState {
    /// The keys a user has pressed for a chord but not yet executed. Think of it as the equivalent
    /// of an HTML <input />'s `value`. Reset to its blocks when executed.
    pub input: ChordInput,

    /// Insert-mode text for each quote in `input`, in order; see `ChordInputEvent::quotes`.
    pub quotes: Vec<String>,

    /// `input` ends with an open quote, so keys are typed into its text instead of
    /// the chord. Chord stays active without Space until Caps+Space closes the quote.
    pub inserting: bool,

    /// Where each block of `input` ends, ascending. Keys typed with Shift held form a
    /// block, which stays in `input` after every run until Backspace removes it.
    pub blocks: Vec<u32>,

    /// Shift is held after typing a key, so the keys after `blocks` are a block that
    /// becomes solid when Shift is released.
    pub open_block: bool,

    /// The chord currently being pressed down.
    pub pressed_input_event: Option<ChordInputEvent>,

    /// The chord that is "active"
    pub selected_input_event: Option<ChordInputEvent>,

    /// Resolved action name for the current buffer or the last executed chord.
    pub description: Option<String>,

    /// Why the selected input failed to execute. Failed input stays selected, so it is
    /// shown like an executed chord until the next key press.
    pub error: Option<String>,
}

impl ChordInputState {
    /// The number of keys at the start of `input` in solid blocks.
    pub fn block_len(&self) -> usize {
        self.blocks
            .last()
            .map_or(0, |&end| (end as usize).min(self.input.len()))
    }

    /// The number of keys at the start of `input` that a run keeps: its blocks,
    /// including one still being typed.
    fn kept_len(&self) -> usize {
        if self.open_block {
            self.input.len()
        } else {
            self.block_len()
        }
    }

    /// Whether `input` has keys after its blocks. Only those make input runnable;
    /// blocks alone never run.
    pub fn has_loose_keys(&self) -> bool {
        self.input.len() > self.kept_len()
    }

    /// The input and quote texts that stay after a run.
    pub fn block_input(&self) -> (ChordInput, Vec<String>) {
        let input = self.input[..self.kept_len()].to_vec();
        let quotes = self
            .quotes
            .iter()
            .take(typed_quote_count(&input))
            .cloned()
            .collect();
        (input, quotes)
    }
}

define_observable!(
    pub struct ChordInputObservable(ChordInputState);
    id: "chord-input";
);

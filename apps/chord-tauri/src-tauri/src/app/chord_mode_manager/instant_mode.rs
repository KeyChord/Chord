use crate::models::{ChordKeyboardEvent, Key, KeyEvent};
use crate::state::ChordInputState;

#[derive(Debug, PartialEq)]
pub(super) enum InstantInput {
    Pass,
    Changed,
    Execute {
        input: Vec<Key>,
        quotes: Vec<String>,
        hold: bool,
    },
}

/// Global-prefix handling shared by the native input entry point and portable tests.
pub(super) fn handle_instant_key_event(
    state: &mut ChordInputState,
    key_event: &KeyEvent,
    event: &ChordKeyboardEvent,
    first_shift: bool,
) -> InstantInput {
    use keycode::KeyMappingCode::*;
    match key_event {
        KeyEvent::Release(Key(ShiftLeft | ShiftRight)) if state.instant_mode.is_some() => {
            if !event.shift_key {
                state.instant_mode = Some(false);
            }
            return InstantInput::Changed;
        }
        _ => {}
    }
    if event.repeat || event.ctrl_key || event.alt_key || event.meta_key {
        return InstantInput::Pass;
    }
    match key_event {
        KeyEvent::Press(Key(ShiftLeft | ShiftRight)) if first_shift => {
            if state.instant_mode.is_some() {
                let run = state.has_loose_keys().then(|| InstantInput::Execute {
                    input: state.input.clone(),
                    quotes: state.quotes.clone(),
                    hold: false,
                });
                let (input, quotes) = state.block_input();
                state.input = input;
                state.quotes = quotes;
                state.instant_mode = Some(true);
                return run.unwrap_or(InstantInput::Changed);
            }
            if is_global_prefix(&state.input) {
                lock_prefix(state);
                return InstantInput::Changed;
            }
        }
        KeyEvent::Press(key) if event.shift_key && !Key::modifiers().contains(key) => {
            // These remain Chord's completion, cancellation and editing controls.
            if matches!(key.0, Escape | Backspace | Tab | Space | CapsLock) {
                return InstantInput::Pass;
            }
            if state.instant_mode == Some(true) && Key::serialize_sequence(&[*key]).is_some() {
                let (mut input, quotes) = state.block_input();
                input.push(*key);
                return InstantInput::Execute {
                    input,
                    quotes,
                    hold: true,
                };
            }
            // Shift can arrive before the first prefix key, including a Shift already
            // held when Caps+Space started Chord. Use sequence characters, not the
            // shifted KeyboardEvent.key ('_' on the physical Minus key).
            if state.input.is_empty() && is_global_prefix(&[*key]) {
                state.input.push(*key);
                lock_prefix(state);
                return InstantInput::Changed;
            }
        }
        _ => {}
    }
    InstantInput::Pass
}

fn is_global_prefix(input: &[Key]) -> bool {
    input
        .first()
        .and_then(Key::to_sequence_char)
        .is_some_and(|character| !character.is_alphanumeric() && !matches!(character, '\'' | '`'))
}

fn lock_prefix(state: &mut ChordInputState) {
    state.blocks = vec![state.input.len() as u32];
    state.open_block = false;
    state.instant_mode = Some(true);
}

#[cfg(test)]
mod tests {
    use super::*;
    use keycode::KeyMappingCode::*;
    use std::collections::HashSet;

    struct Input {
        state: ChordInputState,
        held: HashSet<Key>,
        runs: Vec<Vec<Key>>,
    }

    impl Input {
        fn new(buffer: &str) -> Self {
            Self {
                state: ChordInputState {
                    input: keys(buffer),
                    ..Default::default()
                },
                held: HashSet::new(),
                runs: vec![],
            }
        }

        fn event(&mut self, event: KeyEvent) -> InstantInput {
            let was_shift = self
                .held
                .iter()
                .any(|key| matches!(key.0, ShiftLeft | ShiftRight));
            let repeat = match event {
                KeyEvent::Press(key) => !self.held.insert(key),
                KeyEvent::Release(key) => {
                    self.held.remove(&key);
                    false
                }
            };
            let keyboard = ChordKeyboardEvent::new(
                &event,
                &self.held.iter().copied().collect::<Vec<_>>(),
                repeat,
            );
            let result = handle_instant_key_event(&mut self.state, &event, &keyboard, !was_shift);
            if let InstantInput::Execute { input, .. } = &result {
                self.runs.push(input.clone());
            }
            result
        }

        fn press(&mut self, key: keycode::KeyMappingCode) -> InstantInput {
            self.event(KeyEvent::Press(Key(key)))
        }
        fn release(&mut self, key: keycode::KeyMappingCode) -> InstantInput {
            self.event(KeyEvent::Release(Key(key)))
        }
        fn locked(&self, prefix: &str, held: bool) {
            assert_eq!(self.state.input, keys(prefix));
            assert_eq!(self.state.blocks, vec![keys(prefix).len() as u32]);
            assert_eq!(self.state.instant_mode, Some(held));
            assert!(!self.state.open_block);
        }
    }

    fn keys(input: &str) -> Vec<Key> {
        Key::parse_sequence(input).unwrap()
    }

    #[test]
    fn prefix_then_shift_locks_the_entire_buffer_without_running_it() {
        for prefix in ["-", "-wm", ";w", "/"] {
            let mut input = Input::new(prefix);
            assert_eq!(input.press(ShiftLeft), InstantInput::Changed);
            input.locked(prefix, true);
            assert!(input.runs.is_empty());
            assert!(!input.state.has_loose_keys());
        }
    }

    #[test]
    fn shift_then_prefix_and_shift_carried_into_chord_mode_lock_the_physical_key() {
        for shift in [ShiftLeft, ShiftRight] {
            let mut input = Input::new("");
            assert_eq!(input.press(shift), InstantInput::Pass);
            assert_eq!(input.press(Minus), InstantInput::Changed);
            input.locked("-", true);
            assert!(input.runs.is_empty());
            // Physical Minus remains '-' even though KeyboardEvent.key is '_'.
            let event = ChordKeyboardEvent::new(&KeyEvent::Press(Key(Minus)), &[Key(shift)], false);
            assert_eq!(event.key, "_");
        }
    }

    #[test]
    fn held_keys_execute_once_per_fresh_press_without_buffer_growth() {
        let mut input = Input::new("-w");
        input.press(ShiftLeft);
        for key in [KeyA, KeyA, Semicolon, Slash, Quote, ArrowLeft] {
            let result = input.press(key);
            assert_eq!(
                result,
                InstantInput::Execute {
                    input: [keys("-w"), vec![Key(key)]].concat(),
                    quotes: vec![],
                    hold: true
                }
            );
            input.locked("-w", true);
            assert_eq!(input.press(key), InstantInput::Pass); // autorepeat
            input.release(key);
        }
        assert_eq!(input.runs.len(), 6);
    }

    #[test]
    fn buffered_suffix_runs_exactly_once_and_repeated_cycles_keep_original_prefix() {
        let mut input = Input::new("-w");
        input.press(ShiftLeft);
        for suffix in ["ab", "cd", "ef"] {
            input.release(ShiftLeft);
            assert_eq!(input.state.instant_mode, Some(false));
            // Normal unshifted buffering remains owned by ChordInputManager.
            input.state.input.extend(keys(suffix));
            let result = input.press(ShiftLeft);
            assert_eq!(
                result,
                InstantInput::Execute {
                    input: keys(&format!("-w{suffix}")),
                    quotes: vec![],
                    hold: false
                }
            );
            input.locked("-w", true);
            input.press(ShiftLeft); // modifier autorepeat
            input.press(ShiftRight); // second Shift
            input.release(ShiftLeft); // still holding right Shift
            input.locked("-w", true);
            input.release(ShiftRight);
            input.press(ShiftLeft); // empty suffix
            input.locked("-w", true);
        }
        assert_eq!(input.runs, [keys("-wab"), keys("-wcd"), keys("-wef")]);
    }

    #[test]
    fn both_shift_keys_only_resume_on_transition_from_neither_to_either() {
        let mut input = Input::new("-");
        input.press(ShiftLeft);
        input.press(ShiftRight);
        input.release(ShiftLeft);
        input.locked("-", true);
        input.press(KeyA);
        input.release(KeyA);
        input.release(ShiftRight);
        input.locked("-", false);
        input.state.input.extend(keys("bc"));
        input.press(ShiftRight);
        input.press(ShiftLeft);
        assert_eq!(input.runs, [keys("-a"), keys("-bc")]);
    }

    #[test]
    fn empty_suffix_and_prefix_key_release_never_execute_the_prefix() {
        let mut input = Input::new("");
        input.press(ShiftLeft);
        input.press(Minus);
        input.release(Minus);
        for _ in 0..4 {
            input.release(ShiftLeft);
            input.press(ShiftLeft);
        }
        input.locked("-", true);
        assert!(input.runs.is_empty());
    }

    #[test]
    fn control_keys_and_non_global_inputs_keep_existing_behavior() {
        for prefix in ["", "g", "gd", "0", "12", "'", "`"] {
            let mut input = Input::new(prefix);
            assert_eq!(input.press(ShiftLeft), InstantInput::Pass);
            assert_eq!(input.state.instant_mode, None);
        }
        let mut input = Input::new("-");
        input.press(ShiftLeft);
        for key in [
            Escape,
            Backspace,
            Tab,
            Space,
            CapsLock,
            ControlLeft,
            AltLeft,
            MetaLeft,
        ] {
            assert_eq!(input.press(key), InstantInput::Pass);
            input.release(key);
        }
        assert!(input.runs.is_empty());
        assert_eq!(input.press(ControlLeft), InstantInput::Pass);
        assert_eq!(input.press(KeyA), InstantInput::Pass);
    }

    #[test]
    fn reset_discards_the_lock_and_a_fresh_global_prefix_can_lock_again() {
        let mut input = Input::new("-");
        input.press(ShiftLeft);
        input.state = ChordInputState::default(); // reset / Esc / Shift+Backspace
        input.held.clear();
        assert_eq!(input.press(KeyA), InstantInput::Pass);
        assert_eq!(input.state.instant_mode, None);
        input.state.input = keys("/f");
        input.press(ShiftRight);
        input.locked("/f", true);
    }
}

//! The real ChordInputManager::handle_key_event and buffer helpers, extracted by
//! build.rs without changes. Only app services and execution are simulated.
use crate::{
    instant_mode::{InstantInput, handle_instant_key_event},
    models::*,
    state::ChordInputState,
};
use anyhow::Result;
use keycode::KeyMappingCode;
use parking_lot::Mutex;
use std::{
    collections::HashSet,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum ChordEventTrigger {
    Keydown,
    Keypress,
    Keyup,
    Input,
}

#[derive(Default)]
struct Observable(Mutex<ChordInputState>);
impl Observable {
    fn get_state(&self) -> Result<ChordInputState> {
        Ok(self.0.lock().clone())
    }
    fn set_state(&self, update: impl FnOnce(ChordInputState) -> ChordInputState) -> Result<()> {
        let mut state = self.0.lock();
        *state = update(state.clone());
        Ok(())
    }
}
#[derive(Default)]
struct Sequence {
    macros: ChordMacros,
}
#[derive(Default)]
struct Handle {
    panel: Panel,
}
impl Handle {
    fn app_state(&self) -> &Self {
        self
    }
    fn chord_mode_manager(&self) -> &Self {
        self
    }
}
#[derive(Default)]
struct Panel;
impl Panel {
    fn toggle(&self) -> Result<()> {
        Ok(())
    }
}

#[derive(Default)]
struct ChordInputManager {
    held_keys: Mutex<HashSet<Key>>,
    carried_keys: Mutex<HashSet<Key>>,
    active_task_run: Mutex<Option<(ChordInputEvent, ())>>,
    observable: Observable,
    sequence: Mutex<Sequence>,
    executions: AtomicU64,
    handle: Handle,
    runs: Mutex<Vec<(Vec<Key>, bool, ChordKeyboardEvent)>>,
    fail_next: AtomicBool,
}

impl ChordInputManager {
    fn end_active_task(&self) {
        self.active_task_run.lock().take();
    }
    fn reset_matching_chords(&self) -> Result<()> {
        Ok(())
    }
    fn dispatch_current_buffer(
        &self,
        _: &[ChordEventTrigger],
        _: &ChordKeyboardEvent,
    ) -> Result<()> {
        Ok(())
    }
    fn execute_input(
        &self,
        keys: &[Key],
        quotes: &[String],
        triggers: &[ChordEventTrigger],
        event: &ChordKeyboardEvent,
        hold: bool,
    ) -> Result<()> {
        assert!(triggers.contains(&ChordEventTrigger::Input));
        if matches!(event.code.as_str(), "ShiftLeft" | "ShiftRight") {
            assert!(triggers.contains(&ChordEventTrigger::Keydown));
            assert!(triggers.contains(&ChordEventTrigger::Keypress));
        }
        self.runs.lock().push((keys.to_vec(), hold, event.clone()));
        if self.fail_next.swap(false, Ordering::SeqCst) {
            return self.fail_input(keys, quotes, "No chord found".into());
        }
        self.set_executed_event(ChordInputEvent {
            input: keys.to_vec(),
            quotes: quotes.to_vec(),
            ..Default::default()
        })
    }
}
include!(concat!(env!("OUT_DIR"), "/manager.rs"));

use keycode::KeyMappingCode::*;
fn keys(text: &str) -> Vec<Key> {
    Key::parse_sequence(text).unwrap()
}
fn press(manager: &ChordInputManager, key: KeyMappingCode) {
    manager
        .handle_key_event(&KeyEvent::Press(Key(key)))
        .unwrap();
}
fn release(manager: &ChordInputManager, key: KeyMappingCode) {
    manager
        .handle_key_event(&KeyEvent::Release(Key(key)))
        .unwrap();
}
fn tap(manager: &ChordInputManager, key: KeyMappingCode) {
    press(manager, key);
    release(manager, key);
}
fn input(manager: &ChordInputManager) -> ChordInputState {
    manager.observable.get_state().unwrap()
}
fn ran(manager: &ChordInputManager) -> Vec<String> {
    manager
        .runs
        .lock()
        .iter()
        .map(|(input, ..)| Key::serialize_sequence(input).unwrap())
        .collect()
}
fn text(manager: &ChordInputManager, text: &str) {
    for key in keys(text) {
        tap(manager, key.0);
    }
}

#[test]
fn keyboard_entry_handles_both_orders_and_immediate_actions_without_buffer_growth() {
    for shift_first in [false, true] {
        let manager = ChordInputManager::default();
        if shift_first {
            press(&manager, ShiftLeft);
            tap(&manager, Minus);
        } else {
            tap(&manager, Minus);
            press(&manager, ShiftLeft);
        }
        assert_eq!(input(&manager).input, keys("-"));
        assert_eq!(input(&manager).instant_mode, Some(true));
        for _ in 0..3 {
            press(&manager, KeyA);
            assert_eq!(input(&manager).input, keys("-"));
            press(&manager, KeyA); // held-key repeat does not execute again
            release(&manager, KeyA);
        }
        assert_eq!(ran(&manager), ["-a", "-a", "-a"]);
        assert!(
            manager
                .runs
                .lock()
                .iter()
                .all(|(_, hold, event)| *hold && event.shift_key && !event.repeat)
        );
    }
}

#[test]
fn keyboard_entry_buffers_multiple_keys_and_executes_once_on_shift_across_cycles() {
    let manager = ChordInputManager::default();
    text(&manager, "-wm");
    press(&manager, ShiftLeft);
    for suffix in ["ab", "cd", "ef"] {
        release(&manager, ShiftLeft);
        text(&manager, suffix);
        assert_eq!(input(&manager).input, keys(&format!("-wm{suffix}")));
        press(&manager, ShiftLeft);
        assert_eq!(input(&manager).input, keys("-wm"));
        assert_eq!(input(&manager).blocks, [3]);
        assert_eq!(
            input(&manager).selected_input_event.unwrap().input,
            keys(&format!("-wm{suffix}"))
        );
        press(&manager, ShiftLeft);
        press(&manager, ShiftRight);
        release(&manager, ShiftLeft);
        assert_eq!(input(&manager).instant_mode, Some(true));
        release(&manager, ShiftRight);
        press(&manager, ShiftLeft); // empty suffix
    }
    release(&manager, Space); // prefix alone is not executable
    assert_eq!(ran(&manager), ["-wmab", "-wmcd", "-wmef"]);
    assert!(manager.runs.lock().iter().all(|(_, hold, _)| !hold));
}

#[test]
fn keyboard_entry_supports_shift_carried_in_and_both_near_simultaneous_orders() {
    for order in [[Minus, ShiftLeft], [ShiftLeft, Minus]] {
        let manager = ChordInputManager::default();
        for key in order {
            press(&manager, key);
        }
        release(&manager, Minus);
        tap(&manager, Slash);
        assert_eq!(ran(&manager), ["-/"]);
        assert_eq!(input(&manager).input, keys("-"));
    }
    let manager = ChordInputManager::default();
    manager.begin(vec![Key(ShiftRight)]);
    tap(&manager, Minus);
    tap(&manager, Semicolon);
    assert_eq!(ran(&manager), ["-;"]);
}

#[test]
fn keyboard_entry_preserves_cancel_backspace_reset_and_failed_action_lock() {
    for cancel in [Escape, Backspace] {
        // Shift+Backspace is the existing clear control
        let manager = ChordInputManager::default();
        tap(&manager, Minus);
        press(&manager, ShiftLeft);
        tap(&manager, cancel);
        assert_eq!(input(&manager).instant_mode, None);
        assert!(input(&manager).input.is_empty());
        release(&manager, ShiftLeft);
        text(&manager, "ab");
        release(&manager, Space);
        assert_eq!(ran(&manager), ["ab"]);
    }
    let manager = ChordInputManager::default();
    tap(&manager, Minus);
    press(&manager, ShiftLeft);
    release(&manager, ShiftLeft);
    text(&manager, "ab");
    tap(&manager, Backspace);
    assert_eq!(input(&manager).input, keys("-a"));
    tap(&manager, Backspace); // suffix gone, lock remains
    assert_eq!(input(&manager).instant_mode, Some(false));
    tap(&manager, Backspace); // remove the locked block
    assert_eq!(input(&manager).instant_mode, None);
    tap(&manager, Minus);
    press(&manager, ShiftLeft);
    manager.fail_next.store(true, Ordering::SeqCst);
    tap(&manager, KeyZ);
    assert_eq!(input(&manager).input, keys("-"));
    assert_eq!(input(&manager).instant_mode, Some(true));
    assert!(input(&manager).error.is_some());
    tap(&manager, KeyA);
    assert_eq!(ran(&manager), ["-z", "-a"]);
    assert!(input(&manager).error.is_none());
    manager.reset().unwrap();
    assert_eq!(input(&manager), ChordInputState::default());
}

#[test]
fn keyboard_entry_preserves_local_blocks_meta_config_and_insert_modes() {
    let manager = ChordInputManager::default();
    text(&manager, "g");
    press(&manager, ShiftLeft);
    tap(&manager, KeyD);
    release(&manager, ShiftLeft);
    assert_eq!(input(&manager).blocks, [2]);
    assert_eq!(input(&manager).instant_mode, None);
    text(&manager, "e");
    release(&manager, Space);
    assert_eq!(ran(&manager), ["gde"]);
    assert_eq!(input(&manager).input, keys("gd"));
    for prefix in ["0", "12", "`d5`"] {
        manager.reset().unwrap();
        text(&manager, prefix);
        press(&manager, ShiftLeft);
        assert_eq!(input(&manager).instant_mode, None);
    }
    manager.reset().unwrap();
    press(&manager, ShiftLeft);
    tap(&manager, Quote);
    assert!(input(&manager).inserting);
    assert_eq!(input(&manager).instant_mode, None);
}

#[test]
fn keyboard_entry_preserves_original_lock_when_expanding_suffix_escapes() {
    let manager = ChordInputManager::default();
    text(&manager, "-w");
    press(&manager, ShiftLeft);
    release(&manager, ShiftLeft);
    tap(&manager, Backslash);
    tap(&manager, KeyU);
    tap(&manager, KeyP);
    tap(&manager, Tab);
    assert_eq!(
        input(&manager).input,
        [keys("-w"), vec![Key(ArrowUp)]].concat()
    );
    press(&manager, ShiftLeft);
    assert_eq!(ran(&manager), [r"-w\up"]);
    assert_eq!(input(&manager).input, keys("-w"));
    assert_eq!(input(&manager).blocks, [2]);
}

#[test]
fn keyboard_entry_ignores_non_shift_modifiers_and_keeps_empty_suffix_inert() {
    let manager = ChordInputManager::default();
    tap(&manager, Minus);
    press(&manager, ShiftLeft);
    for modifier in [ControlLeft, MetaLeft, AltLeft, Fn] {
        press(&manager, modifier);
        tap(&manager, KeyA);
        release(&manager, modifier);
        assert!(ran(&manager).is_empty());
        assert_eq!(input(&manager).input, keys("-"));
    }
    for _ in 0..4 {
        release(&manager, ShiftLeft);
        press(&manager, ShiftLeft);
    }
    release(&manager, Space);
    assert!(ran(&manager).is_empty());
}

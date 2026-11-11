//! Caps Lock behaviour while Chord is its only writer.
//!
//! The physical key is remapped (see `caps_remap`), so macOS never toggles Caps Lock
//! on its own and the state read when the key goes down is exactly the state before
//! the press. Chord decides every change:
//! - A plain tap (no Space while held) toggles Caps Lock. Turning it off happens at
//!   key-down, like macOS; turning it on waits for release, because at key-down Chord
//!   cannot know whether Space will follow and make the press an activation.
//! - Only a press made with Caps Lock off can be an activation. With Caps Lock on, the
//!   press turns it off and a Space that follows is typed: someone writing in capitals
//!   ends the word that way (see `AppKeyboardState::arm_caps`).
//! - Normal mode keeps Caps Lock off, and Chord's exit restores the state from before
//!   it started. Toggles made while typing in insert mode are not kept.

use parking_lot::Mutex;

#[derive(Default)]
struct State {
    /// The state before the current press, while it may still be a plain tap.
    before_press: Option<bool>,
    /// The state from before Chord started, restored when Chord ends.
    saved: Option<bool>,
}

#[derive(Default)]
pub struct CapsLockGuard(Mutex<State>);

impl CapsLockGuard {
    /// Caps went down. `current` is the lock state now; returns the state to apply.
    pub fn pressed(&self, current: bool, in_normal_mode: bool) -> Option<bool> {
        let mut state = self.0.lock();
        if in_normal_mode {
            // Caps runs the chord in normal mode; it never toggles.
            state.before_press = None;
            return None;
        }
        state.before_press = Some(current);
        current.then_some(false)
    }

    /// Caps+Space started normal mode, from idle or from insert mode. The press was
    /// an activation, not a tap; returns the state to apply (always off).
    pub fn normal_mode_started(&self, current: bool) -> bool {
        let mut state = self.0.lock();
        let before = state.before_press.take().unwrap_or(current);
        // Returning from insert mode keeps the state from before Chord started.
        state.saved.get_or_insert(before);
        false
    }

    /// Caps went up. Returns the state to apply when the press was a plain tap.
    pub fn released(&self) -> Option<bool> {
        let before = self.0.lock().before_press.take()?;
        // Turning off already happened at key-down.
        (!before).then_some(true)
    }

    /// Chord ended, or input ownership moved: returns the state from before Chord
    /// started, and forgets the current press.
    pub fn chord_ended(&self) -> Option<bool> {
        let mut state = self.0.lock();
        state.before_press = None;
        state.saved.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Applies the guard's decisions to a simulated lock state. With the key remapped,
    /// nothing else changes it.
    struct Computer {
        guard: CapsLockGuard,
        caps_lock: bool,
    }

    impl Computer {
        fn new(caps_lock: bool) -> Self {
            Self {
                guard: CapsLockGuard::default(),
                caps_lock,
            }
        }
        fn apply(&mut self, change: Option<bool>) {
            if let Some(enabled) = change {
                self.caps_lock = enabled;
            }
        }
        fn press(&mut self, in_normal_mode: bool) {
            let change = self.guard.pressed(self.caps_lock, in_normal_mode);
            self.apply(change);
        }
        fn release(&mut self) {
            let change = self.guard.released();
            self.apply(change);
        }
        fn start_normal_mode(&mut self) {
            let change = self.guard.normal_mode_started(self.caps_lock);
            self.apply(Some(change));
        }
        fn end_chord(&mut self) {
            let change = self.guard.chord_ended();
            self.apply(change);
        }
    }

    #[test]
    fn plain_taps_toggle_turning_off_at_key_down_and_on_at_release() {
        let mut computer = Computer::new(false);
        computer.press(false);
        assert!(!computer.caps_lock, "turning on waits for release");
        computer.release();
        assert!(computer.caps_lock);
        computer.press(false);
        assert!(!computer.caps_lock, "turning off is immediate");
        computer.release();
        assert!(!computer.caps_lock);
    }

    #[test]
    fn the_activating_press_never_changes_caps_lock_and_chord_restores_it() {
        for before in [false, true] {
            let mut computer = Computer::new(before);
            computer.press(false);
            computer.start_normal_mode();
            assert!(!computer.caps_lock, "normal mode keeps Caps Lock off");
            computer.release();
            assert!(!computer.caps_lock, "the activation was not a tap");
            // Running chords with Caps never toggles it.
            for _ in 0..3 {
                computer.press(true);
                computer.release();
                assert!(!computer.caps_lock);
            }
            computer.end_chord();
            assert_eq!(computer.caps_lock, before);
            // Afterwards, taps behave normally again.
            computer.press(false);
            computer.release();
            assert_eq!(computer.caps_lock, !before);
        }
    }

    #[test]
    fn releasing_space_before_caps_still_restores_and_leaves_no_tap_behind() {
        for before in [false, true] {
            let mut computer = Computer::new(before);
            computer.press(false);
            computer.start_normal_mode();
            computer.end_chord();
            computer.release();
            assert_eq!(computer.caps_lock, before);
        }
    }

    #[test]
    fn insert_mode_toggles_freely_and_its_toggles_are_not_kept() {
        for before in [false, true] {
            let mut computer = Computer::new(before);
            computer.press(false);
            computer.start_normal_mode();
            computer.release();
            // Insert mode: plain taps toggle.
            computer.press(false);
            computer.release();
            assert!(computer.caps_lock);
            // Caps+Space returns to normal mode: off, and the earlier state is kept.
            computer.press(false);
            computer.start_normal_mode();
            computer.release();
            assert!(!computer.caps_lock);
            computer.press(false);
            computer.release();
            assert!(computer.caps_lock);
            computer.end_chord();
            assert_eq!(computer.caps_lock, before);
        }
    }
}

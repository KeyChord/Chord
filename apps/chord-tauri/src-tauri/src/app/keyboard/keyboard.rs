use super::caps_lock_guard::CapsLockGuard;
use super::caps_remap::{ProcessIdentity, RemapStore, SystemMappingTable};
use super::caps_space_gap::{CapsSpaceGaps, KeyTime, milliseconds};
use super::health::{Backoff, InputHealth};
use super::secure_input::SecureInputState;
use super::tap::{TapAction, TapEvent, TapHandle};
use super::{caps_guardian, caps_hotkey, key_catcher};
use crate::app::AppHandleExt;
use crate::app::permissions::InputPermissions;
use crate::models::{AppKeyboardState, CapturedKeyEvent, Key, KeyEvent, KeyEventAction};
use anyhow::Result;
use bitflags::bitflags;
use keycode::KeyMappingCode;
use nject::injectable;
use objc2_core_graphics::{CGEventFlags, CGEventSource, CGEventSourceStateID};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex, MutexGuard, Once, PoisonError};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

const KEY_EVENT_QUEUE_CAPACITY: usize = 1_024;

/// How often the supervisor re-checks permissions and listener health.
pub const SUPERVISOR_INTERVAL: Duration = Duration::from_millis(250);

bitflags! {
  pub struct Modifiers: u16 {
      const LEFT_SHIFT      = 1 << 0;
      const RIGHT_SHIFT     = 1 << 1;
      const LEFT_CONTROL    = 1 << 2;
      const RIGHT_CONTROL   = 1 << 3;
      const LEFT_OPTION     = 1 << 4;
      const RIGHT_OPTION    = 1 << 5;
      const LEFT_COMMAND    = 1 << 6;
      const RIGHT_COMMAND   = 1 << 7;
      const FUNCTION        = 1 << 8;
  }
}

/// What the listeners hand to the thread that runs the controller.
enum InputMessage {
    Key(CapturedKeyEvent),
    /// Ends Chord mode, in order with the keys queued before it.
    CancelChord(&'static str),
}

type InputQueue = SyncSender<(u64, InputMessage)>;

/// The listener threads currently owned by the supervisor.
#[derive(Default)]
pub(super) struct Listeners {
    tap: Option<TapHandle>,
    tap_backoff: Backoff,
    secure_input: SecureInputState,
}

#[injectable]
pub struct AppKeyboard {
    keyboard_state: AppKeyboardState,

    #[inject(AtomicU16::new(0))]
    pub modifier_flags: AtomicU16,

    #[inject(Arc::new(InputHealth::default()))]
    health: Arc<InputHealth>,

    #[inject(Mutex::new(Listeners::default()))]
    listeners: Mutex<Listeners>,

    #[inject(CapsLockGuard::default())]
    caps_lock_guard: CapsLockGuard,

    /// Whether this instance holds the Caps key mapping (see `caps_remap`).
    #[inject(AtomicBool::new(false))]
    holds_caps_remap: AtomicBool,

    /// A tap replacement is under way (see `on_caps_event_missed_by_tap`).
    #[inject(AtomicBool::new(false))]
    tap_restart_pending: AtomicBool,

    /// The queue to the controller thread, shared by every source of key events: the
    /// tap, the hotkey fallback and the key catcher.
    #[inject(Mutex::new(None))]
    input_queue: Mutex<Option<InputQueue>>,

    #[inject(AtomicBool::new(false))]
    queue_trouble_reported: AtomicBool,

    /// Chord is reading the chord in progress through its own key window, because the
    /// tap cannot see it (see `key_catcher`).
    #[inject(AtomicBool::new(false))]
    catching_keys: AtomicBool,

    /// How soon Space follows Caps on this person's keyboard (see `caps_space_gap`).
    #[inject(CapsSpaceGaps::default())]
    caps_space_gaps: CapsSpaceGaps,

    /// The Caps press in progress went through the tap, so its release is the tap's
    /// too. A press that arrived as a hotkey has its release left to the hotkey.
    #[inject(AtomicBool::new(false))]
    tap_has_caps_press: AtomicBool,

    handle: AppHandle,
}

impl AppKeyboard {
    pub fn state(&self) -> &AppKeyboardState {
        &self.keyboard_state
    }

    pub fn health(&self) -> &InputHealth {
        &self.health
    }

    /// The event tap is up. Input ownership (see `DevLockfileDetector`) is only
    /// claimed while this holds; the Caps key reaches Chord through the tap.
    pub fn input_handlers_running(&self) -> bool {
        self.health.tap_running()
    }

    /// Input ownership changed hands: put back the Caps Lock state from before
    /// Chord started, and forget every held key.
    pub fn reset_input_state(&self) {
        self.on_chord_mode_ended();
        self.keyboard_state.reset();
        self.modifier_flags.store(0, Ordering::Relaxed);
    }

    /// Takes or gives up the Caps key: its mapping, and the hotkey that covers for
    /// the tap under Secure Input. Only the input owner holds them, so a standby
    /// instance leaves Caps Lock alone. Called by `DevLockfileDetector`.
    pub fn set_input_owner(&self, active: bool) {
        if active {
            if let Err(error) = self.install_caps_remap() {
                log::error!(
                    "Failed to take over the Caps key; Caps+Space cannot start Chord: {error:#}"
                );
            }
        } else {
            self.release_caps_remap();
        }
        let handle = self.handle.clone();
        let registered = self.handle.run_on_main_thread(move || {
            if let Err(error) = caps_hotkey::set_registered(&handle, active) {
                log::warn!("Failed to update the Caps hotkey fallback: {error:#}");
            }
        });
        if let Err(error) = registered {
            log::warn!("Failed to schedule the Caps hotkey fallback update: {error:#}");
        }
    }

    /// Chord is exiting: restore Caps Lock and give the key back to macOS.
    pub fn release_input(&self) {
        self.on_chord_mode_ended();
        self.release_caps_remap();
        if let Some(summary) = self.caps_space_gaps.summary() {
            log::info!("Caps+Space gaps this run: {summary}");
        }
    }

    fn caps_remap_store(&self) -> Result<RemapStore> {
        // Shared by every channel, like the input lease, so one guardian sees any holder.
        let directory = self
            .handle
            .path()
            .local_data_dir()?
            .join("com.leonsilicon.chord/caps-remap-v1");
        Ok(RemapStore::new(directory.join("state.json")))
    }

    fn install_caps_remap(&self) -> Result<()> {
        let store = self.caps_remap_store()?;
        static GUARDIAN: Once = Once::new();
        GUARDIAN.call_once(|| {
            let identifier = &self.handle.config().identifier;
            if let Err(error) = caps_guardian::ensure_registered(identifier, store.path()) {
                log::error!(
                    "Failed to register the Caps guardian; a crash would leave Caps Lock remapped: {error:#}"
                );
            }
        });
        store.install(&SystemMappingTable, ProcessIdentity::current()?)?;
        if !self.holds_caps_remap.swap(true, Ordering::SeqCst) {
            log::info!("Took over the Caps key (mapped to F20 while Chord owns input)");
        }
        Ok(())
    }

    fn release_caps_remap(&self) {
        if !self.holds_caps_remap.swap(false, Ordering::SeqCst) {
            return;
        }
        let released = self.caps_remap_store().and_then(|store| {
            store.uninstall(&SystemMappingTable, Some(ProcessIdentity::current()?))
        });
        match released {
            Ok(_) => log::info!("Gave the Caps key back to macOS"),
            Err(error) => log::error!("Failed to remove the Caps key mapping: {error:#}"),
        }
    }

    /// macOS toggled Caps Lock itself, so something removed Chord's mapping (UTM's
    /// "Caps Lock as key" cleanup does). Put it back; the toggle stands, like any tap.
    fn on_native_caps_toggle(&self) {
        if !self.holds_caps_remap.load(Ordering::SeqCst) {
            return;
        }
        log::warn!(
            "macOS toggled Caps Lock itself; the Caps key mapping was removed by another app. Restoring it"
        );
        let handle = self.handle.clone();
        let spawned = std::thread::Builder::new()
            .name("chord-caps-remap".into())
            .spawn(move || {
                let keyboard = handle.app_state().keyboard();
                if keyboard.holds_caps_remap.load(Ordering::SeqCst)
                    && let Err(error) = keyboard.install_caps_remap()
                {
                    log::error!("Failed to restore the Caps key mapping: {error:#}");
                }
            });
        if let Err(error) = spawned {
            log::error!("Failed to restore the Caps key mapping: {error:#}");
        }
    }

    /// Caps+Space started Chord's normal mode, from idle or from insert mode.
    /// Caps Lock stays off while normal mode is active. Called by the controller.
    pub fn on_normal_mode_started(&self) {
        let current = Self::get_caps_lock()
            .inspect_err(|error| log::warn!("Failed to read the Caps Lock toggle: {error:#}"))
            .unwrap_or(false);
        let enabled = self.caps_lock_guard.normal_mode_started(current);
        Self::apply_caps_lock(enabled);
    }

    /// Chord ended: put back the Caps Lock toggle from before it started.
    /// Called by the controller.
    pub fn on_chord_mode_ended(&self) {
        if let Some(enabled) = self.caps_lock_guard.chord_ended() {
            Self::apply_caps_lock(enabled);
        }
        self.stop_catching_keys();
    }

    fn apply_caps_lock(enabled: bool) {
        log::debug!("Setting Caps Lock to {enabled}");
        if let Err(error) = Self::set_caps_lock(enabled) {
            log::warn!("Failed to set the Caps Lock toggle: {error:#}");
        }
    }

    /// Starts the supervisor thread that owns listener registration for the
    /// lifetime of the process. Permissions are polled here as well, so a
    /// grant made in System Settings is picked up without the settings UI.
    pub(super) fn start_supervisor(&self) -> Result<()> {
        let handle = self.handle.clone();
        std::thread::Builder::new()
            .name("chord-input-supervisor".into())
            .spawn(move || {
                let mut permissions_error_reported = false;
                loop {
                    let state = handle.app_state();
                    let permissions = match state.permissions().refresh_if_changed() {
                        Ok(permissions) => {
                            permissions_error_reported = false;
                            permissions
                        }
                        Err(error) => {
                            if !permissions_error_reported {
                                permissions_error_reported = true;
                                log::error!("Failed to refresh input permissions: {error:#}");
                            }
                            InputPermissions::UNAVAILABLE
                        }
                    };
                    state
                        .keyboard()
                        .reconcile_listeners(permissions, Instant::now());
                    state.keyboard().check_secure_input();
                    std::thread::sleep(SUPERVISOR_INTERVAL);
                }
            })?;
        Ok(())
    }

    fn listeners(&self) -> MutexGuard<'_, Listeners> {
        self.listeners
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Brings the listeners in line with the granted permissions, restarting
    /// anything that died. All non-granted results fail closed.
    pub(super) fn reconcile_listeners(&self, permissions: InputPermissions, now: Instant) {
        let mut listeners = self.listeners();
        self.reconcile_tap(&mut listeners, permissions.accessibility, now);
    }

    fn reconcile_tap(&self, listeners: &mut Listeners, accessibility: bool, now: Instant) {
        if listeners.tap.as_ref().is_some_and(TapHandle::is_finished) {
            log::error!("Keyboard event tap thread exited unexpectedly; scheduling a restart");
            if let Some(tap) = listeners.tap.take() {
                tap.stop();
            }
            listeners.tap_backoff.failed(now);
        }
        if !accessibility {
            if let Some(tap) = listeners.tap.take() {
                log::warn!(
                    "Accessibility check no longer reports granted; stopping the keyboard event tap"
                );
                tap.stop();
            }
            return;
        }
        if listeners.tap.is_some() || !listeners.tap_backoff.ready(now) {
            return;
        }
        self.start_tap(listeners, now);
    }

    fn start_tap(&self, listeners: &mut Listeners, now: Instant) {
        match self.spawn_tap() {
            Ok(tap) => {
                if listeners.tap_backoff.failures() > 0 {
                    log::info!(
                        "Keyboard event tap recovered after {} failed attempt(s)",
                        listeners.tap_backoff.failures()
                    );
                }
                listeners.tap_backoff.succeeded();
                listeners.tap = Some(tap);
            }
            Err(error) => {
                let delay = listeners.tap_backoff.failed(now);
                log::error!(
                    "Failed to start the keyboard event tap (retrying in {delay:?}): {error:#}"
                );
            }
        }
    }

    fn check_secure_input(&self) {
        let current = SecureInputState::current();
        let mut listeners = self.listeners();
        if listeners.secure_input == current {
            return;
        }
        match &current {
            SecureInputState::Disabled => {
                log::warn!("Secure Input is disabled again; the event tap sees every key")
            }
            enabled => log::warn!(
                "Secure Input is {}; macOS hides keys from the event tap, so Caps+Space goes through the hotkey fallback until it is disabled",
                enabled.describe()
            ),
        }
        let turned_on = listeners.secure_input == SecureInputState::Disabled;
        listeners.secure_input = current;
        drop(listeners);
        // A chord the tap was reading can no longer end: its keys, and the release of
        // Space, are hidden from now on. Without this Chord mode would stay on and
        // swallow keys once the tap sees them again.
        if turned_on && !self.catching_keys.load(Ordering::SeqCst) {
            self.cancel_chord("Secure Input was enabled during the chord");
        }
        // The same for a Caps press the tap was following: its release is hidden, and
        // Caps would read as held, and Space as starting Chord, from then on.
        if turned_on && self.tap_has_caps_press.swap(false, Ordering::SeqCst) {
            self.ingest_as_owner(KeyEvent::Release(Key(KeyMappingCode::CapsLock)), None);
        }
    }

    /// The queue to the thread that runs the controller, started on first use. Keeping
    /// listener callbacks minimal: controller work runs there instead.
    fn input_queue(&self) -> Result<InputQueue> {
        let mut queue = self
            .input_queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(queue) = queue.as_ref() {
            return Ok(queue.clone());
        }
        let (tx, rx) = sync_channel::<(u64, InputMessage)>(KEY_EVENT_QUEUE_CAPACITY);
        let handle = self.handle.clone();
        std::thread::Builder::new()
            .name("chord-key-events".into())
            .spawn(move || {
                while let Ok((generation, message)) = rx.recv() {
                    let state = handle.app_state();
                    state.dev_lockfile_detector().with_input_generation(
                        generation,
                        || match message {
                            InputMessage::Key(event) => {
                                // In order with the controller, which decides the mode.
                                state.keyboard().handle_caps_event(&event.event);
                                if let Err(error) = state.app_controller().handle_key_event(&event)
                                {
                                    log::error!("Failed to handle key event: {error:#}");
                                }
                            }
                            InputMessage::CancelChord(reason) => {
                                if !state.app_controller().app_mode().is_chord() {
                                    return;
                                }
                                log::warn!("Ending the chord in progress: {reason}");
                                if let Err(error) = state.app_controller().set_idle_mode() {
                                    log::error!("Failed to end the chord: {error:#}");
                                }
                            }
                        },
                    );
                }
            })?;
        *queue = Some(tx.clone());
        Ok(tx)
    }

    fn enqueue(&self, queue: &InputQueue, generation: u64, message: InputMessage) {
        let problem = match queue.try_send((generation, message)) {
            Ok(()) => return,
            Err(TrySendError::Full(_)) => {
                "Dropping keyboard events because the bounded input queue is saturated"
            }
            Err(TrySendError::Disconnected(_)) => "Keyboard event handler thread is gone",
        };
        if !self.queue_trouble_reported.swap(true, Ordering::SeqCst) {
            log::error!("{problem}");
        }
    }

    /// Takes one key event from any listener: updates the tracked keyboard state,
    /// decides whether the event reaches other applications, and queues it for the
    /// controller. `caps_lock_on` comes with a Caps press: the lock state before it.
    /// `event_ns` is the event's own timestamp, from the listeners that have one.
    /// Runs on the listener threads, so it must stay cheap.
    fn ingest(
        &self,
        queue: &InputQueue,
        generation: u64,
        event: KeyEvent,
        caps_lock_on: Option<bool>,
        event_ns: Option<u64>,
    ) -> KeyEventAction {
        let time = KeyTime {
            event_ns,
            arrived: Instant::now(),
        };
        let caps_released = event == KeyEvent::Release(Key(KeyMappingCode::CapsLock));
        let input = CapturedKeyEvent {
            event,
            caps_armed: self.keyboard_state.is_caps_armed(),
        };
        let action = self.handle_key_event(&input);
        if let Some(caps_lock_on) = caps_lock_on {
            self.keyboard_state.arm_caps(caps_lock_on);
            if caps_lock_on {
                self.caps_space_gaps.caps_cleared();
            } else {
                self.caps_space_gaps.caps_pressed(time);
            }
        }
        if input.activates_chord_mode() {
            self.note_caps_space_gap(time);
        }
        self.enqueue(queue, generation, InputMessage::Key(input));
        if caps_released {
            self.caps_space_gaps.caps_cleared();
            // Whichever listener saw the release, the hotkey fallback stops waiting
            // for a Space after this press.
            caps_hotkey::caps_released(&self.handle);
        }
        action
    }

    /// Space started Chord: records how soon after Caps it came. A new shortest gap
    /// and a summary now and then go to the log; the rest would crowd it out.
    fn note_caps_space_gap(&self, at: KeyTime) {
        let Some(gap) = self.caps_space_gaps.space_followed(at) else {
            return;
        };
        let (space_after, arrival) = (milliseconds(gap.gap), milliseconds(gap.arrival_gap));
        log::debug!("Caps+Space: Space followed Caps by {space_after} (arrived {arrival} apart)");
        if !gap.shortest_so_far && !gap.summary_due {
            return;
        }
        let summary = self.caps_space_gaps.summary().unwrap_or_default();
        if gap.shortest_so_far {
            log::info!(
                "Caps+Space: Space followed Caps by {space_after} (arrived {arrival} apart), the shortest so far; {summary}"
            );
        } else {
            log::info!("Caps+Space gaps so far: {summary}");
        }
    }

    /// Like `ingest`, for the listeners that are not under the event tap's deadline.
    /// Returns `None` when this instance does not own input.
    fn ingest_as_owner(&self, event: KeyEvent, caps_lock_on: Option<bool>) -> Option<()> {
        let queue = self
            .input_queue()
            .inspect_err(|error| log::error!("Failed to start the key event queue: {error:#}"))
            .ok()?;
        self.handle
            .app_state()
            .dev_lockfile_detector()
            .with_input_owner(|generation| {
                self.ingest(&queue, generation, event, caps_lock_on, None);
            })
    }

    /// Ends the chord in progress, in order with the keys already queued.
    fn cancel_chord(&self, reason: &'static str) {
        if !self
            .handle
            .app_state()
            .app_controller()
            .app_mode()
            .is_chord()
        {
            return;
        }
        let Ok(queue) = self.input_queue() else {
            return;
        };
        let detector = self.handle.app_state().dev_lockfile_detector();
        detector.with_input_owner(|generation| {
            self.enqueue(&queue, generation, InputMessage::CancelChord(reason));
        });
    }

    /// Creates the tap, which feeds the controller queue.
    fn spawn_tap(&self) -> Result<TapHandle> {
        let queue = self.input_queue()?;
        let handle = self.handle.clone();
        let sink = move |event: TapEvent, event_ns: u64| -> TapAction {
            let keyboard = handle.app_state().keyboard();
            handle
                .app_state()
                .dev_lockfile_detector()
                .with_input_owner(|generation| {
                    let (event, caps_lock_on) = match event {
                        // A key that went down as a hotkey must come up as one: macOS
                        // keeps a hotkey pressed until it sees the release, and reports
                        // no further press of it. This happens when Secure Input ends
                        // between the two, as it does when the app that held it loses
                        // the keyboard to Chord's key window.
                        TapEvent::Key(KeyEvent::Release(Key(KeyMappingCode::CapsLock)))
                            if !keyboard.tap_has_caps_press.swap(false, Ordering::SeqCst) =>
                        {
                            return TapAction::Forward;
                        }
                        TapEvent::Key(KeyEvent::Release(Key(KeyMappingCode::Space)))
                            if caps_hotkey::space_press_taken() =>
                        {
                            return TapAction::Forward;
                        }
                        TapEvent::Key(event) => (event, None),
                        TapEvent::CapsPressed { caps_lock_on } => {
                            keyboard.tap_has_caps_press.store(true, Ordering::SeqCst);
                            (
                                KeyEvent::Press(Key(KeyMappingCode::CapsLock)),
                                Some(caps_lock_on),
                            )
                        }
                        TapEvent::RemappedCapsRepeat => return TapAction::Consume,
                        TapEvent::NativeCapsToggle => {
                            keyboard.on_native_caps_toggle();
                            return TapAction::Forward;
                        }
                        // Only the input owner rewrites, so two instances never both do.
                        TapEvent::CapsLockChanged => return TapAction::ForwardAsCapsLockKey,
                        TapEvent::CapsLockRepeated => return TapAction::Consume,
                    };
                    keyboard
                        .ingest(&queue, generation, event, caps_lock_on, Some(event_ns))
                        .into()
                })
                .unwrap_or(TapAction::Forward)
        };
        TapHandle::spawn(self.health.clone(), Box::new(sink))
    }

    pub fn emit_caps_lock() -> Result<()> {
        super::caps_lock::toggle()
    }

    pub fn set_caps_lock_off() -> Result<()> {
        Self::set_caps_lock(false)
    }

    pub fn set_caps_lock(enabled: bool) -> Result<()> {
        super::caps_lock::set(enabled)
    }

    pub fn get_caps_lock() -> Result<bool> {
        super::caps_lock::get()
    }

    /// Updates the tracked keyboard state and decides whether the event
    /// reaches other applications. Runs on the listener threads, so it must
    /// stay cheap.
    fn handle_key_event(&self, input: &CapturedKeyEvent) -> KeyEventAction {
        let event = &input.event;
        log::debug!("Processing event {event:?}");
        self.keyboard_state.handle_key_event(event);
        self.update_modifier_flags(event);

        let app_mode = self.handle.app_state().app_controller().app_mode();

        // Space while Caps is held enters Chord mode and is never typed. The Caps
        // key itself is Chord's alone: it arrives remapped and never reaches apps.
        if input.activates_chord_mode()
            || matches!(
                event,
                KeyEvent::Press(Key(KeyMappingCode::CapsLock))
                    | KeyEvent::Release(Key(KeyMappingCode::CapsLock))
            )
        {
            return KeyEventAction::Consume;
        }

        // While Chord reads a chord through its key window, every key is the chord's,
        // including in the instant before the controller has switched modes.
        if app_mode.is_chord() || self.catching_keys.load(Ordering::SeqCst) {
            return KeyEventAction::Consume;
        }

        KeyEventAction::Forward
    }

    /// Applies Caps Lock for a Caps press or release, in order with the controller.
    /// See `CapsLockGuard` for the rules.
    fn handle_caps_event(&self, event: &KeyEvent) {
        match event {
            KeyEvent::Press(Key(KeyMappingCode::CapsLock)) => {
                let state = self.handle.app_state();
                let in_normal_mode = state.app_controller().app_mode().is_chord()
                    && !state.chord_mode_manager().is_inserting();
                let current = match Self::get_caps_lock() {
                    Ok(current) => current,
                    Err(error) => {
                        log::warn!("Failed to read the Caps Lock toggle: {error:#}");
                        return;
                    }
                };
                if let Some(enabled) = self.caps_lock_guard.pressed(current, in_normal_mode) {
                    Self::apply_caps_lock(enabled);
                }
            }
            KeyEvent::Release(Key(KeyMappingCode::CapsLock)) => {
                if let Some(enabled) = self.caps_lock_guard.released() {
                    Self::apply_caps_lock(enabled);
                }
            }
            _ => {}
        }
    }

    /// A Caps press or release that got past the tap, delivered by the hotkey
    /// fallback instead. The tap swallows the key whenever it receives it, so the
    /// hotkey doubles as a witness: `tap_received` is false when the tap never got
    /// this event. Called on the main thread.
    pub(super) fn handle_unseen_caps_event(&self, event: KeyEvent, tap_received: bool) {
        let pressed = matches!(event, KeyEvent::Press(_));
        if pressed {
            // This press is the hotkey's, and so is its release.
            self.tap_has_caps_press.store(false, Ordering::SeqCst);
        }
        // The lock state before the press, as on the event the tap would have seen.
        let caps_lock_on = pressed.then(Self::caps_lock_flag);
        self.ingest_as_owner(event, caps_lock_on);
        if !tap_received {
            self.on_caps_event_missed_by_tap();
        }
    }

    /// Whether the keyboard reports Caps Lock as on. Unlike the key-state query, the
    /// modifier flags stay correct under Secure Input.
    fn caps_lock_flag() -> bool {
        CGEventSource::flags_state(CGEventSourceStateID::HIDSystemState)
            .contains(CGEventFlags::MaskAlphaShift)
    }

    /// The tap is registered but did not receive the Caps key. Under Secure Input
    /// macOS hides every key from it, and the hotkey fallback takes over; otherwise the
    /// tap is dead in a way macOS does not report (it still reads as enabled), so
    /// replace it.
    fn on_caps_event_missed_by_tap(&self) {
        if SecureInputState::current() != SecureInputState::Disabled {
            return;
        }
        if self.tap_restart_pending.swap(true, Ordering::SeqCst) {
            return;
        }
        log::error!(
            "The keyboard event tap did not receive the Caps key although Secure Input is off; replacing the tap"
        );
        let handle = self.handle.clone();
        let spawned = std::thread::Builder::new()
            .name("chord-tap-restart".into())
            .spawn(move || {
                let keyboard = handle.app_state().keyboard();
                keyboard.restart_tap(Instant::now());
                keyboard.tap_restart_pending.store(false, Ordering::SeqCst);
            });
        if let Err(error) = spawned {
            self.tap_restart_pending.store(false, Ordering::SeqCst);
            log::error!("Failed to replace the keyboard event tap: {error:#}");
        }
    }

    /// Space went down while the hotkey fallback had it registered: the tap is blind
    /// and Caps is held. Returns whether the press is Chord's; otherwise the caller
    /// gives it back to the focused app. Called on the main thread.
    pub(super) fn handle_unseen_space_press(&self) -> bool {
        let armed = self.keyboard_state.is_caps_armed();
        let in_chord = self
            .handle
            .app_state()
            .app_controller()
            .app_mode()
            .is_chord();
        if !space_press_is_chords(armed, in_chord) {
            return false;
        }
        if armed {
            // Before the event is queued: the keys that follow Space must already
            // find Chord's window, and the tap must stay out of the way.
            self.start_catching_keys();
        }
        self.ingest_as_owner(KeyEvent::Press(Key(KeyMappingCode::Space)), None)
            .is_some()
    }

    /// The Space press Chord took through the hotkey fallback ended. Called on the
    /// main thread.
    pub(super) fn handle_unseen_space_release(&self) {
        // The controller ends the chord, or keeps it for insert mode, in order with
        // the keys before this one; the keyboard goes back when it does.
        self.ingest_as_owner(KeyEvent::Release(Key(KeyMappingCode::Space)), None);
    }

    /// A key that reached Chord's key window (see `key_catcher`). Called on the main
    /// thread.
    pub(super) fn handle_caught_event(&self, event: TapEvent) {
        match event {
            TapEvent::Key(event) => {
                self.ingest_as_owner(event, None);
            }
            TapEvent::CapsPressed { caps_lock_on } => {
                self.ingest_as_owner(
                    KeyEvent::Press(Key(KeyMappingCode::CapsLock)),
                    Some(caps_lock_on),
                );
            }
            // Lock-state reports and repeats of the Caps key mean nothing to a chord.
            TapEvent::RemappedCapsRepeat
            | TapEvent::NativeCapsToggle
            | TapEvent::CapsLockChanged
            | TapEvent::CapsLockRepeated => {}
        }
    }

    /// The key window lost the keyboard to another window while a chord was being
    /// read through it. Called on the main thread.
    pub(super) fn on_key_catcher_lost_focus(&self) {
        self.cancel_chord("another window took the keyboard from Chord");
    }

    /// Whether Chord is reading the chord in progress through its own key window.
    pub fn is_catching_keys(&self) -> bool {
        self.catching_keys.load(Ordering::SeqCst)
    }

    /// Takes the keyboard for the chord that is starting, by giving a window of Chord's
    /// own the key focus. The app in front stays in front. Called on the main thread.
    fn start_catching_keys(&self) {
        if self.catching_keys.swap(true, Ordering::SeqCst) {
            return;
        }
        // The OS key-state query is blind too, so the held keys are kept from events:
        // the modifiers and Caps now, then every key that reaches the window.
        let mut held = super::tap::held_modifier_keys(
            CGEventSource::flags_state(CGEventSourceStateID::HIDSystemState).bits(),
        );
        if self.keyboard_state.is_caps_pressed() {
            held.push(Key(KeyMappingCode::CapsLock));
        }
        self.keyboard_state.track_keys(held);
        if let Err(error) = key_catcher::show(&self.handle) {
            log::error!("Could not take the keyboard for the chord: {error:#}");
        }
    }

    /// Gives the keyboard back to the app that had it. Safe from any thread, and when
    /// nothing was taken.
    fn stop_catching_keys(&self) {
        if self.catching_keys.swap(false, Ordering::SeqCst) {
            self.keyboard_state.stop_tracking_keys();
            if objc2_foundation::MainThreadMarker::new().is_some() {
                key_catcher::hide();
            } else if let Err(error) = self.handle.run_on_main_thread(key_catcher::hide) {
                log::error!("Failed to give the keyboard back after the chord: {error:#}");
            }
        }
        caps_hotkey::chord_ended(&self.handle);
    }

    /// Replaces a running tap with a new one. Without a tap there is nothing to
    /// replace: the supervisor starts one as soon as permissions allow.
    fn restart_tap(&self, now: Instant) {
        let mut listeners = self.listeners();
        let Some(tap) = listeners.tap.take() else {
            return;
        };
        // Input ownership follows the tap; the swap must not look like losing it.
        self.health.set_tap_restarting(true);
        tap.stop();
        self.start_tap(&mut listeners, now);
        self.health.set_tap_restarting(false);
    }

    #[allow(dead_code)]
    pub fn get_modifier_flags(&self) -> Modifiers {
        Modifiers::from_bits_truncate(self.modifier_flags.load(Ordering::Relaxed))
    }

    fn modifier_key_to_flag(key: &Key) -> Option<Modifiers> {
        let flag = match key.0 {
            KeyMappingCode::ShiftLeft => Modifiers::LEFT_SHIFT,
            KeyMappingCode::ShiftRight => Modifiers::RIGHT_SHIFT,
            KeyMappingCode::ControlLeft => Modifiers::LEFT_CONTROL,
            KeyMappingCode::ControlRight => Modifiers::RIGHT_CONTROL,
            KeyMappingCode::AltLeft => Modifiers::LEFT_OPTION,
            KeyMappingCode::AltRight => Modifiers::RIGHT_OPTION,
            KeyMappingCode::MetaLeft => Modifiers::LEFT_COMMAND,
            KeyMappingCode::MetaRight => Modifiers::RIGHT_COMMAND,
            KeyMappingCode::Fn => Modifiers::FUNCTION,
            _ => return None,
        };

        Some(flag)
    }

    fn update_modifier_flags(&self, event: &KeyEvent) {
        match event {
            KeyEvent::Press(key) => {
                if let Some(flag) = Self::modifier_key_to_flag(key) {
                    self.modifier_flags.fetch_or(flag.bits(), Ordering::Relaxed);
                }
            }
            KeyEvent::Release(key) => {
                if let Some(flag) = Self::modifier_key_to_flag(key) {
                    self.modifier_flags
                        .fetch_and(!flag.bits(), Ordering::Relaxed);
                }
            }
        }
    }
}

/// Whether a Space press that reached the hotkey fallback belongs to Chord: it follows
/// an armed Caps press, or a chord is being read and Space is one of its keys.
fn space_press_is_chords(caps_armed: bool, in_chord: bool) -> bool {
    caps_armed || in_chord
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn space_is_chords_only_after_an_armed_caps_press_or_inside_a_chord() {
        assert!(space_press_is_chords(true, false));
        assert!(space_press_is_chords(false, true));
        assert!(space_press_is_chords(true, true));
        // A stale registration: the press is handed back to the focused app.
        assert!(!space_press_is_chords(false, false));
    }
}

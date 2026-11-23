use super::caps_lock_guard::CapsLockGuard;
use super::caps_remap::{
    self, CHORD_MAPPING, MappingTable, ProcessIdentity, RemapStore, SystemMappingTable,
};
use super::caps_space_gap::{CapsSpaceGaps, KeyTime, milliseconds};
use super::health::{Backoff, InputHealth};
use super::secure_input::SecureInputState;
use super::tap::{TapAction, TapEvent, TapHandle};
use super::thread_priority::{InputThreadPriority, raise_current_thread};
use super::{caps_guardian, caps_hotkey, key_catcher};
use crate::app::AppHandleExt;
use crate::app::controller::AppController;
use crate::app::dev_lockfile_detector::DevLockfileDetector;
use crate::app::permissions::InputPermissions;
use crate::models::{AppKeyboardState, CapturedKeyEvent, Key, KeyEvent, KeyEventAction};
use anyhow::Result;
use bitflags::bitflags;
use keycode::KeyMappingCode;
use nject::injectable;
use objc2_core_graphics::{CGEventFlags, CGEventSource, CGEventSourceStateID};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex, MutexGuard, Once, OnceLock, PoisonError};
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

/// The singletons the event tap consults for every key, looked up once. Tauri finds
/// managed state under a process-wide mutex that every thread of the app shares, and
/// on macOS a std mutex does not lend its waiter's priority to its holder: a
/// background thread preempted inside it on a busy machine would hold up the tap
/// thread, and with it every key, until macOS gave up on the tap.
struct InputSingletons {
    keyboard: &'static AppKeyboard,
    detector: &'static DevLockfileDetector,
    controller: &'static AppController,
}

fn input_singletons(handle: &AppHandle) -> &'static InputSingletons {
    // The handle is kept for good, so the state it manages lives as long.
    static HANDLE: OnceLock<AppHandle> = OnceLock::new();
    static SINGLETONS: OnceLock<InputSingletons> = OnceLock::new();
    SINGLETONS.get_or_init(|| {
        let state = HANDLE.get_or_init(|| handle.clone()).app_state();
        InputSingletons {
            keyboard: state.keyboard(),
            detector: state.dev_lockfile_detector(),
            controller: state.app_controller(),
        }
    })
}

/// How many times Chord puts its Caps mapping back within `RESTORE_WINDOW` before it
/// stops: another tool that re-applies its own mapping on every change would otherwise
/// trade writes with Chord forever.
const RESTORE_LIMIT: usize = 5;
const RESTORE_WINDOW: Duration = Duration::from_secs(10);

/// Recent restorations of the Caps mapping (see `RESTORE_LIMIT`).
#[derive(Default)]
struct RestoreBudget {
    recent: Vec<Instant>,
    exhausted_reported: bool,
}

#[derive(Debug, PartialEq)]
enum RestoreDecision {
    Restore,
    /// Over the limit; `report` is true the first time.
    GiveUp { report: bool },
}

impl RestoreBudget {
    fn spend(&mut self, now: Instant) -> RestoreDecision {
        self.recent
            .retain(|at| now.duration_since(*at) < RESTORE_WINDOW);
        if self.recent.len() >= RESTORE_LIMIT {
            let report = !std::mem::replace(&mut self.exhausted_reported, true);
            return RestoreDecision::GiveUp { report };
        }
        self.recent.push(now);
        self.exhausted_reported = false;
        RestoreDecision::Restore
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

    #[inject(Mutex::new(RestoreBudget::default()))]
    restore_budget: Mutex<RestoreBudget>,

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
            // Before the first install, so no change to the table goes unseen.
            static HANDLE: OnceLock<AppHandle> = OnceLock::new();
            let _ = HANDLE.set(self.handle.clone());
            let watching = caps_remap::watch_system_table(|| {
                if let Some(handle) = HANDLE.get() {
                    handle.app_state().keyboard().on_mapping_table_changed();
                }
            });
            if let Err(error) = watching {
                log::error!(
                    "Failed to watch the Caps key mapping; one removed by another app is only noticed at the next Caps press: {error:#}"
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

    /// macOS toggled Caps Lock itself, so something removed Chord's mapping and the
    /// change notification has not put it back yet. The toggle stands, like any tap.
    fn on_native_caps_toggle(&self) {
        self.restore_caps_remap("macOS toggled Caps Lock itself");
    }

    /// Some process wrote the key mapping table (UTM's "Caps Lock as key" cleanup
    /// clears it, for one). Called on the main thread, for Chord's own writes too.
    fn on_mapping_table_changed(&self) {
        self.restore_caps_remap("another app changed the key mapping table");
    }

    /// Puts Chord's Caps mapping back if this instance holds it and it is gone. Reads
    /// the table and takes the state lock, so the work happens on its own thread.
    fn restore_caps_remap(&self, noticed: &'static str) {
        if !self.holds_caps_remap.load(Ordering::SeqCst) {
            return;
        }
        let handle = self.handle.clone();
        let spawned = std::thread::Builder::new()
            .name("chord-caps-remap".into())
            .spawn(move || {
                let keyboard = handle.app_state().keyboard();
                if !keyboard.holds_caps_remap.load(Ordering::SeqCst) {
                    return;
                }
                match SystemMappingTable.read() {
                    Ok(entries) if caps_mapping_is_chords(&entries) => return,
                    Ok(_) => {}
                    Err(error) => {
                        log::error!("Failed to read the Caps key mapping: {error:#}");
                        return;
                    }
                }
                let decision = keyboard
                    .restore_budget
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .spend(Instant::now());
                match decision {
                    RestoreDecision::Restore => {}
                    RestoreDecision::GiveUp { report } => {
                        if report {
                            log::error!(
                                "Another app keeps replacing Chord's Caps key mapping ({RESTORE_LIMIT} times in {RESTORE_WINDOW:?}); leaving it until that stops. Caps+Space cannot start Chord meanwhile"
                            );
                        }
                        return;
                    }
                }
                log::warn!(
                    "The Caps key mapping was removed by another app ({noticed}). Restoring it"
                );
                if let Err(error) = keyboard.install_caps_remap() {
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
                // Decides the mode and runs the chord: what a key press is waiting for.
                raise_current_thread(InputThreadPriority::Initiated);
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
        // Nothing in here may wait on a lock another thread can be preempted in.
        let InputSingletons {
            keyboard, detector, ..
        } = input_singletons(&self.handle);
        let sink = move |event: TapEvent, event_ns: u64| -> TapAction {
            detector
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

        let app_mode = input_singletons(&self.handle).controller.app_mode();

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
    /// this event. `late` is how long the key took to get here. Called on the main
    /// thread.
    pub(super) fn handle_unseen_caps_event(
        &self,
        event: KeyEvent,
        tap_received: bool,
        late: Duration,
    ) {
        let pressed = matches!(event, KeyEvent::Press(_));
        if pressed {
            // This press is the hotkey's, and so is its release.
            self.tap_has_caps_press.store(false, Ordering::SeqCst);
        }
        // The lock state before the press, as on the event the tap would have seen.
        let caps_lock_on = pressed.then(Self::caps_lock_flag);
        self.ingest_as_owner(event, caps_lock_on);
        // Under Secure Input macOS hides every key from the tap, and the hotkey
        // fallback takes over; that is no fault of the tap.
        if tap_received || SecureInputState::current() != SecureInputState::Disabled {
            return;
        }
        self.on_caps_event_missed_by_tap(late);
        if pressed {
            self.start_chord_if_space_got_ahead();
        }
    }

    /// A Caps press the tap missed reached Chord late, with Secure Input off: the tap
    /// was disabled or the process stalled. Nothing held back the keys behind it, so a
    /// Space that followed went to the focused app instead of starting Chord. If Space
    /// is still down, start Chord now: the stray space stays typed, but the chord the
    /// person is about to type runs instead of being typed as well.
    ///
    /// The keyboard's own key state stands in for the event Chord never received.
    /// It cannot tell a Space pressed before Caps from one pressed after; the first
    /// starts Chord here and not through the tap, which only matters during an outage.
    fn start_chord_if_space_got_ahead(&self) {
        if !self.keyboard_state.is_caps_armed()
            || input_singletons(&self.handle).controller.app_mode().is_chord()
            || !CGEventSource::key_state(
                CGEventSourceStateID::HIDSystemState,
                super::tap::SPACE_KEYCODE as u16,
            )
        {
            return;
        }
        log::warn!(
            "Space was already down when the missed Caps press reached Chord; starting Chord from the keys held"
        );
        self.ingest_as_owner(KeyEvent::Press(Key(KeyMappingCode::Space)), None);
    }

    /// The modifier keys held right now, by the keyboard's own flags. They include
    /// keys Chord consumed and stay correct under Secure Input.
    pub fn held_modifier_keys() -> Vec<Key> {
        super::tap::held_modifier_keys(
            CGEventSource::flags_state(CGEventSourceStateID::HIDSystemState).bits(),
        )
    }

    /// Whether the keyboard reports Caps Lock as on. Unlike the key-state query, the
    /// modifier flags stay correct under Secure Input.
    fn caps_lock_flag() -> bool {
        CGEventSource::flags_state(CGEventSourceStateID::HIDSystemState)
            .contains(CGEventFlags::MaskAlphaShift)
    }

    /// The tap is registered but did not receive the Caps key, with Secure Input off.
    /// Either macOS disabled it, or it is dead in a way macOS does not report (it
    /// still reads as enabled). Replace it either way.
    fn on_caps_event_missed_by_tap(&self, late: Duration) {
        if self.tap_restart_pending.swap(true, Ordering::SeqCst) {
            return;
        }
        log::error!(
            "The keyboard event tap did not receive the Caps key although Secure Input is off (macOS reports the tap {}; the key reached Chord {late:?} after it was pressed); replacing the tap",
            if self.health.tap_enabled() {
                "enabled"
            } else {
                "disabled"
            }
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
        if let Err(error) = key_catcher::show(&self.handle) {
            log::error!("Could not take the keyboard for the chord: {error:#}");
        }
    }

    /// Gives the keyboard back to the app that had it. Safe from any thread, and when
    /// nothing was taken.
    fn stop_catching_keys(&self) {
        if self.catching_keys.swap(false, Ordering::SeqCst) {
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

/// Whether the table maps the Caps key to Chord's key and to nothing else.
fn caps_mapping_is_chords(entries: &[caps_remap::Mapping]) -> bool {
    caps_remap::with_chord_mapping(entries).0 == entries && entries.contains(&CHORD_MAPPING)
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

    #[test]
    fn only_a_table_with_chords_caps_entry_alone_needs_no_restoring() {
        let other = (0x7_0000_0004, 0x7_0000_0005);
        assert!(caps_mapping_is_chords(&[CHORD_MAPPING]));
        assert!(caps_mapping_is_chords(&[other, CHORD_MAPPING]));
        // Cleared by another tool, or never set.
        assert!(!caps_mapping_is_chords(&[]));
        assert!(!caps_mapping_is_chords(&[other]));
        // Caps mapped elsewhere, with or without Chord's entry.
        let caps_elsewhere = (caps_remap::CAPS_USAGE, 0x7_0000_00E0);
        assert!(!caps_mapping_is_chords(&[caps_elsewhere]));
        assert!(!caps_mapping_is_chords(&[caps_elsewhere, CHORD_MAPPING]));
    }

    #[test]
    fn restoring_stops_when_another_app_keeps_undoing_it() {
        let mut budget = RestoreBudget::default();
        let start = Instant::now();
        for _ in 0..RESTORE_LIMIT {
            assert_eq!(budget.spend(start), RestoreDecision::Restore);
        }
        // Reported once, however long the other app keeps at it.
        assert_eq!(
            budget.spend(start),
            RestoreDecision::GiveUp { report: true }
        );
        assert_eq!(
            budget.spend(start + Duration::from_secs(1)),
            RestoreDecision::GiveUp { report: false }
        );
        // Once the window has passed, Chord tries again.
        let later = start + RESTORE_WINDOW;
        assert_eq!(budget.spend(later), RestoreDecision::Restore);
        for _ in 1..RESTORE_LIMIT {
            assert_eq!(budget.spend(later), RestoreDecision::Restore);
        }
        assert_eq!(
            budget.spend(later),
            RestoreDecision::GiveUp { report: true }
        );
    }
}

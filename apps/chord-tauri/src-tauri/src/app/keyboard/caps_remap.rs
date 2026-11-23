//! Chord owns Caps Lock by remapping the physical key with macOS's `UserKeyMapping`.
//!
//! The HID event system applies `UserKeyMapping` before it toggles Caps Lock, so a key
//! mapped away from Caps (usage 0x39) never changes the lock state and Chord becomes its
//! only writer. The key reaches Chord's event tap as F20, ordered with every other key.
//!
//! The mapping is one system-wide table that outlives the process that set it. Chord
//! edits only its own entry, and records the owner and any Caps entries it displaced in
//! a state file. The crash guardian reads that file to undo the mapping if Chord dies.

use anyhow::{Context, Result, ensure};
use objc2_core_foundation::{CFArray, CFDictionary, CFNumber, CFRetained, CFString, CFType};
use objc2_io_kit::IOHIDEventSystemClient;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

/// HID usage of the physical Caps Lock key.
pub const CAPS_USAGE: u64 = 0x7_0000_0039;
/// HID usage of F20, which Caps is remapped to. F18 is taken by other remappers.
pub const REMAPPED_USAGE: u64 = 0x7_0000_006F;
/// macOS virtual keycode of F20 (`kVK_F20`), as the event tap sees the remapped key.
pub const REMAPPED_KEYCODE: i64 = 0x5A;

const USER_KEY_MAPPING: &str = "UserKeyMapping";
const MAPPING_SRC: &str = "HIDKeyboardModifierMappingSrc";
const MAPPING_DST: &str = "HIDKeyboardModifierMappingDst";

/// A `UserKeyMapping` entry: source usage to destination usage.
pub type Mapping = (u64, u64);

pub const CHORD_MAPPING: Mapping = (CAPS_USAGE, REMAPPED_USAGE);

/// Installs Chord's entry. Other entries for the Caps key are displaced and returned, so
/// they can be restored; every unrelated entry is kept as is. A table that already has
/// Chord's entry and no other Caps entry comes back unchanged.
pub fn with_chord_mapping(entries: &[Mapping]) -> (Vec<Mapping>, Vec<Mapping>) {
    let (mut kept, displaced): (Vec<_>, Vec<_>) = entries
        .iter()
        .partition(|entry| entry.0 != CAPS_USAGE || **entry == CHORD_MAPPING);
    if !kept.contains(&CHORD_MAPPING) {
        kept.push(CHORD_MAPPING);
    }
    (kept, displaced)
}

/// Removes Chord's entry and puts back what it displaced, unless the Caps key has
/// been mapped again by someone else meanwhile.
pub fn without_chord_mapping(entries: &[Mapping], displaced: &[Mapping]) -> Vec<Mapping> {
    let mut kept: Vec<_> = entries
        .iter()
        .copied()
        .filter(|entry| *entry != CHORD_MAPPING)
        .collect();
    if !kept.iter().any(|(src, _)| *src == CAPS_USAGE) {
        kept.extend_from_slice(displaced);
    }
    kept
}

/// Who holds Chord's mapping, and what to restore when it is removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemapState {
    pub owner_pid: u32,
    /// Start time of the owner, so a recycled PID is never mistaken for Chord.
    pub owner_started: u64,
    pub displaced: Vec<Mapping>,
}

impl RemapState {
    pub fn holder(&self) -> ProcessIdentity {
        ProcessIdentity {
            pid: self.owner_pid,
            started: self.owner_started,
        }
    }
}

/// The state file and the lock that serializes Chord and its guardian.
pub struct RemapStore {
    path: PathBuf,
}

impl RemapStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Runs `operation` while holding an exclusive lock shared with the guardian.
    pub fn locked<T>(&self, operation: impl FnOnce(&Self) -> Result<T>) -> Result<T> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.path.with_extension("lock"))
            .context("failed to open the Caps remap lock")?;
        // SAFETY: a valid descriptor; the lock is released when the file closes.
        ensure!(
            unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) } == 0,
            "failed to lock the Caps remap state: {}",
            std::io::Error::last_os_error()
        );
        let result = operation(self);
        drop(lock);
        result
    }

    pub fn read(&self) -> Result<Option<RemapState>> {
        match std::fs::read(&self.path) {
            Ok(bytes) => Ok(Some(
                serde_json::from_slice(&bytes).context("invalid Caps remap state")?,
            )),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error).context("failed to read the Caps remap state"),
        }
    }

    /// Atomic replace, so the guardian never reads a partial file.
    pub fn write(&self, state: &RemapState) -> Result<()> {
        let temporary = self.path.with_extension("tmp");
        let mut file = File::create(&temporary)?;
        file.write_all(&serde_json::to_vec(state)?)?;
        file.sync_all()?;
        std::fs::rename(&temporary, &self.path).context("failed to save the Caps remap state")
    }

    pub fn remove(&self) -> Result<()> {
        match std::fs::remove_file(&self.path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
            _ => Ok(()),
        }
    }

    /// Installs Chord's mapping for `owner`. A state left by a dead owner keeps its
    /// displaced entries, which belong to the user, not to Chord. Idempotent: nothing
    /// is written when the mapping and state are already in place.
    pub fn install(&self, table: &impl MappingTable, owner: ProcessIdentity) -> Result<()> {
        self.locked(|store| {
            let previous = store.read()?;
            let current = table.read()?;
            let (entries, displaced) = with_chord_mapping(&current);
            let displaced = match &previous {
                Some(previous) if displaced.is_empty() => previous.displaced.clone(),
                _ => displaced,
            };
            let state = RemapState {
                owner_pid: owner.pid,
                owner_started: owner.started,
                displaced,
            };
            // Record first: if Chord dies right after the write, the guardian can undo it.
            if previous.as_ref() != Some(&state) {
                store.write(&state)?;
            }
            if entries != current {
                table.write(&entries)?;
            }
            Ok(())
        })
    }

    /// Removes Chord's mapping when `owner` holds it or its holder has died. With no
    /// owner (the guardian), only a dead holder's mapping is removed.
    pub fn uninstall(
        &self,
        table: &impl MappingTable,
        owner: Option<ProcessIdentity>,
    ) -> Result<bool> {
        self.locked(|store| {
            let Some(state) = store.read()? else {
                return Ok(false);
            };
            let holder = state.holder();
            if Some(holder) != owner && holder.is_alive() {
                return Ok(false);
            }
            table.write(&without_chord_mapping(&table.read()?, &state.displaced))?;
            store.remove()?;
            Ok(true)
        })
    }
}

/// The system-wide `UserKeyMapping`, abstracted so the logic above is testable.
pub trait MappingTable {
    fn read(&self) -> Result<Vec<Mapping>>;
    fn write(&self, entries: &[Mapping]) -> Result<()>;
}

/// macOS's table. Set without a service, it applies to every keyboard, including ones
/// connected later.
pub struct SystemMappingTable;

impl MappingTable for SystemMappingTable {
    fn read(&self) -> Result<Vec<Mapping>> {
        let client = IOHIDEventSystemClient::new_simple_client(None);
        let Some(value) = client.property(&CFString::from_static_str(USER_KEY_MAPPING)) else {
            return Ok(vec![]);
        };
        let array = value
            .downcast::<CFArray>()
            .map_err(|_| anyhow::anyhow!("UserKeyMapping is not an array"))?;
        // SAFETY: UserKeyMapping is documented as an array of dictionaries.
        let array = unsafe { CFRetained::cast_unchecked::<CFArray<CFType>>(array) };
        let src_key = CFString::from_static_str(MAPPING_SRC);
        let dst_key = CFString::from_static_str(MAPPING_DST);
        let mut entries = Vec::new();
        for item in array.iter() {
            let Ok(dictionary) = item.downcast::<CFDictionary>() else {
                continue;
            };
            // SAFETY: mapping entries are dictionaries keyed by strings.
            let dictionary =
                unsafe { CFRetained::cast_unchecked::<CFDictionary<CFString, CFType>>(dictionary) };
            let number = |key: &CFString| {
                dictionary
                    .get(key)
                    .and_then(|value| value.downcast_ref::<CFNumber>().and_then(CFNumber::as_i64))
            };
            if let (Some(src), Some(dst)) = (number(&src_key), number(&dst_key)) {
                entries.push((src as u64, dst as u64));
            }
        }
        Ok(entries)
    }

    fn write(&self, entries: &[Mapping]) -> Result<()> {
        let src_key = CFString::from_static_str(MAPPING_SRC);
        let dst_key = CFString::from_static_str(MAPPING_DST);
        let dictionaries: Vec<CFRetained<CFDictionary<CFString, CFNumber>>> = entries
            .iter()
            .map(|(src, dst)| {
                CFDictionary::from_slices(
                    &[&*src_key, &*dst_key],
                    &[
                        &*CFNumber::new_i64(*src as i64),
                        &*CFNumber::new_i64(*dst as i64),
                    ],
                )
            })
            .collect();
        let array = CFArray::from_retained_objects(&dictionaries);
        let client = IOHIDEventSystemClient::new_simple_client(None);
        // SAFETY: a property list value of the documented shape.
        let applied = unsafe {
            client.set_property(&CFString::from_static_str(USER_KEY_MAPPING), array.as_ref())
        };
        ensure!(applied, "macOS rejected the Caps Lock key mapping");
        Ok(())
    }
}

type TableChanged = unsafe extern "C-unwind" fn(
    target: *mut std::ffi::c_void,
    context: *mut std::ffi::c_void,
    property: *const CFString,
    value: *const CFType,
);

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C-unwind" {
    fn IOHIDEventSystemClientRegisterPropertyChangedCallback(
        client: *const IOHIDEventSystemClient,
        property: *const CFString,
        callback: TableChanged,
        target: *mut std::ffi::c_void,
        context: *mut std::ffi::c_void,
    );
    fn IOHIDEventSystemClientScheduleWithRunLoop(
        client: *const IOHIDEventSystemClient,
        run_loop: *const objc2_core_foundation::CFRunLoop,
        mode: *const CFString,
    );
}

static TABLE_CHANGED: std::sync::OnceLock<fn()> = std::sync::OnceLock::new();

unsafe extern "C-unwind" fn table_changed(
    _target: *mut std::ffi::c_void,
    _context: *mut std::ffi::c_void,
    _property: *const CFString,
    _value: *const CFType,
) {
    if let Some(changed) = TABLE_CHANGED.get() {
        // A panic must never unwind into IOKit.
        let _ = std::panic::catch_unwind(changed);
    }
}

/// Calls `changed` on the main thread each time any process writes `UserKeyMapping`,
/// Chord included. macOS reports the write within milliseconds (measured: 6 ms), so a
/// mapping another tool removed is put back before the next Caps press instead of
/// being discovered by losing one. Only the first call registers.
pub fn watch_system_table(changed: fn()) -> Result<()> {
    if TABLE_CHANGED.set(changed).is_err() {
        return Ok(());
    }
    let run_loop =
        objc2_core_foundation::CFRunLoop::main().context("the main run loop is unavailable")?;
    let client = IOHIDEventSystemClient::new_simple_client(None);
    // SAFETY: a live client, static strings and a static callback; the client is
    // leaked below, so it outlives every notification.
    unsafe {
        IOHIDEventSystemClientRegisterPropertyChangedCallback(
            &*client,
            &*CFString::from_static_str(USER_KEY_MAPPING),
            table_changed,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        IOHIDEventSystemClientScheduleWithRunLoop(
            &*client,
            &*run_loop,
            objc2_core_foundation::kCFRunLoopCommonModes
                .map_or(std::ptr::null(), |mode| mode as *const CFString),
        );
    }
    std::mem::forget(client);
    Ok(())
}

/// A process, identified by PID and start time so a recycled PID never matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessIdentity {
    pub pid: u32,
    pub started: u64,
}

impl ProcessIdentity {
    pub fn current() -> Result<Self> {
        let pid = std::process::id();
        Ok(Self {
            pid,
            started: process_start(pid).context("failed to read Chord's start time")?,
        })
    }

    pub fn is_alive(&self) -> bool {
        process_start(self.pid) == Some(self.started)
    }
}

/// Start time (seconds) of a live process, or `None` when it no longer exists.
pub fn process_start(pid: u32) -> Option<u64> {
    let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::zeroed();
    let size = std::mem::size_of::<libc::proc_bsdinfo>() as libc::c_int;
    // SAFETY: the buffer is exactly one proc_bsdinfo, as PROC_PIDTBSDINFO expects.
    let written = unsafe {
        libc::proc_pidinfo(
            pid as libc::c_int,
            libc::PROC_PIDTBSDINFO,
            0,
            info.as_mut_ptr().cast(),
            size,
        )
    };
    // SAFETY: proc_pidinfo filled the structure when it returned its full size.
    (written == size).then(|| unsafe { info.assume_init() }.pbi_start_tvsec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    const F18: Mapping = (0x7_0000_0039, 0x7_0000_006D);
    const OTHER: Mapping = (0x7_0000_0004, 0x7_0000_0005);

    #[derive(Default)]
    struct Table {
        entries: RefCell<Vec<Mapping>>,
        writes: std::cell::Cell<usize>,
    }

    impl MappingTable for Table {
        fn read(&self) -> Result<Vec<Mapping>> {
            Ok(self.entries.borrow().clone())
        }
        fn write(&self, entries: &[Mapping]) -> Result<()> {
            *self.entries.borrow_mut() = entries.to_vec();
            self.writes.set(self.writes.get() + 1);
            Ok(())
        }
    }

    fn store() -> RemapStore {
        let directory = std::env::temp_dir().join(format!("chord-remap-{}", uuid::Uuid::new_v4()));
        RemapStore::new(directory.join("caps-remap.json"))
    }

    #[test]
    fn chord_displaces_other_caps_entries_and_keeps_unrelated_ones() {
        assert_eq!(with_chord_mapping(&[]), (vec![CHORD_MAPPING], vec![]));
        assert_eq!(
            with_chord_mapping(&[OTHER, F18]),
            (vec![OTHER, CHORD_MAPPING], vec![F18])
        );
        // Installing twice never duplicates, displaces or moves Chord's entry.
        assert_eq!(
            with_chord_mapping(&[CHORD_MAPPING, OTHER]),
            (vec![CHORD_MAPPING, OTHER], vec![])
        );
        assert_eq!(
            with_chord_mapping(&[CHORD_MAPPING, F18]),
            (vec![CHORD_MAPPING], vec![F18])
        );
        assert_eq!(
            without_chord_mapping(&[OTHER, CHORD_MAPPING], &[F18]),
            vec![OTHER, F18]
        );
        // A Caps mapping someone set meanwhile wins over the displaced one.
        let later = (CAPS_USAGE, 0x7_0000_0029);
        assert_eq!(
            without_chord_mapping(&[CHORD_MAPPING, later], &[F18]),
            vec![later]
        );
    }

    #[test]
    fn install_records_the_owner_before_mapping_and_uninstall_restores() {
        let table = Table::default();
        table.write(&[OTHER, F18]).unwrap();
        let store = store();
        let me = ProcessIdentity::current().unwrap();
        store.install(&table, me).unwrap();
        assert_eq!(table.read().unwrap(), [OTHER, CHORD_MAPPING]);
        let state = store.read().unwrap().unwrap();
        assert_eq!(
            (state.owner_pid, state.displaced.clone()),
            (me.pid, vec![F18])
        );
        // Re-installing keeps what was originally displaced and rewrites nothing.
        let writes = table.writes.get();
        store.install(&table, me).unwrap();
        assert_eq!(store.read().unwrap().unwrap().displaced, [F18]);
        assert_eq!(table.writes.get(), writes);
        assert!(store.uninstall(&table, Some(me)).unwrap());
        assert_eq!(table.read().unwrap(), [OTHER, F18]);
        assert_eq!(store.read().unwrap(), None);
        assert!(!store.uninstall(&table, Some(me)).unwrap());
    }

    #[test]
    fn a_live_owner_keeps_the_mapping_while_a_dead_one_releases_it() {
        let table = Table::default();
        let store = store();
        let me = ProcessIdentity::current().unwrap();
        // The test runner's parent stands in for another live instance; spawning one
        // would hold other tests' file locks open across fork.
        // SAFETY: no arguments.
        let parent = unsafe { libc::getppid() } as u32;
        let other = ProcessIdentity {
            pid: parent,
            started: process_start(parent).unwrap(),
        };
        // A live instance cannot take the mapping this process holds, and neither
        // can the guardian.
        store.install(&table, me).unwrap();
        assert!(!store.uninstall(&table, Some(other)).unwrap());
        assert!(!store.uninstall(&table, None).unwrap());
        assert_eq!(table.read().unwrap(), [CHORD_MAPPING]);
        store.install(&table, other).unwrap();
        assert!(!store.uninstall(&table, Some(me)).unwrap());
        // Once its holder is gone, anyone releases it.
        let gone = ProcessIdentity {
            pid: u32::MAX >> 1,
            started: 0,
        };
        assert!(!gone.is_alive());
        store.install(&table, gone).unwrap();
        assert!(store.uninstall(&table, None).unwrap());
        assert_eq!(table.read().unwrap(), []);
        assert_eq!(store.read().unwrap(), None);
    }

    #[test]
    fn process_identity_rejects_recycled_and_missing_pids() {
        let me = ProcessIdentity::current().unwrap();
        assert!(me.is_alive());
        assert!(
            !ProcessIdentity {
                started: me.started.wrapping_sub(1),
                ..me
            }
            .is_alive()
        );
        assert_eq!(process_start(u32::MAX >> 1), None);
    }
}

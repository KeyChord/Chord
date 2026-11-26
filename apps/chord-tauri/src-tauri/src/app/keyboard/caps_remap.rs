//! Chord owns Caps Lock by remapping the physical key with macOS's `UserKeyMapping`.
//!
//! The HID event system applies `UserKeyMapping` before it toggles Caps Lock, so a key
//! mapped away from Caps (usage 0x39) never changes the lock state and Chord becomes its
//! only writer. The key reaches Chord's event tap as F20, ordered with every other key.
//!
//! macOS keeps one table per keyboard, plus a default that a keyboard starts from when
//! it connects. A key does what its own keyboard's table says, and any process may
//! write either kind (UTM clears each keyboard's table and leaves the default alone).
//! So "Chord owns Caps" means one thing only: every connected keyboard's table has
//! Chord's entry. Chord reads and writes all of them (`MappingTables`) and never takes
//! the default as proof of what a keyboard does.
//!
//! The tables outlive the process that set them. Chord edits only its own entry, and
//! records the owner and any Caps entries it displaced in a state file. The crash
//! guardian reads that file to undo the mapping if Chord dies.

use anyhow::{Context, Result, ensure};
use objc2_core_foundation::{CFArray, CFDictionary, CFNumber, CFRetained, CFString, CFType};
use objc2_io_kit::{IOHIDEventSystemClient, IOHIDServiceClient};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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

/// Whether a table maps the Caps key to Chord's key and to nothing else.
pub fn is_chords(entries: &[Mapping]) -> bool {
    with_chord_mapping(entries).0 == entries
}

/// Which of the tables that decide what a key does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableId {
    /// What a keyboard starts from when it connects.
    Default,
    /// A connected keyboard, by its IORegistry entry ID.
    Keyboard(u64),
}

/// One table as it is right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub id: TableId,
    /// For the log: "the default table" or the keyboard's product name.
    pub name: String,
    pub entries: Vec<Mapping>,
}

/// The names of the tables in which the Caps key is not Chord's alone. Empty exactly
/// when Caps reaches Chord from every keyboard.
pub fn tables_without_chord_mapping(tables: &impl MappingTables) -> Result<Vec<String>> {
    Ok(tables
        .read()?
        .into_iter()
        .filter(|table| !is_chords(&table.entries))
        .map(|table| table.name)
        .collect())
}

/// Who holds Chord's mapping, and what to restore when it is removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemapState {
    pub owner_pid: u32,
    /// Start time of the owner, so a recycled PID is never mistaken for Chord.
    pub owner_started: u64,
    pub displaced: Vec<Mapping>,
    /// Original Caps entries for keyboards already connected when Chord took over.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    keyboard_displaced: BTreeMap<u64, Vec<Mapping>>,
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

    /// Installs Chord's mapping for `owner` in every table. A state left by a dead
    /// owner keeps its displaced entries, which belong to the user, not to Chord.
    /// Idempotent: a table or state that is already right is not written.
    pub fn install(&self, tables: &impl MappingTables, owner: ProcessIdentity) -> Result<()> {
        self.locked(|store| {
            let previous = store.read()?;
            let current = tables.read()?;
            let displaced = current
                .iter()
                .find(|table| table.id == TableId::Default)
                .map(|table| with_chord_mapping(&table.entries).1)
                .unwrap_or_default();
            let displaced = match &previous {
                Some(previous) if displaced.is_empty() => previous.displaced.clone(),
                _ => displaced,
            };
            let mut keyboard_displaced = previous
                .as_ref()
                .map(|previous| previous.keyboard_displaced.clone())
                .unwrap_or_default();
            for table in &current {
                if let TableId::Keyboard(id) = table.id {
                    let (_, displaced) = with_chord_mapping(&table.entries);
                    if previous.is_none() || !displaced.is_empty() {
                        keyboard_displaced.insert(id, displaced);
                    }
                }
            }
            let state = RemapState {
                owner_pid: owner.pid,
                owner_started: owner.started,
                displaced,
                keyboard_displaced,
            };
            // Record first: if Chord dies right after the write, the guardian can undo it.
            if previous.as_ref() != Some(&state) {
                store.write(&state)?;
            }
            write_tables(tables, &current, "map", |table| {
                with_chord_mapping(&table.entries).0
            })
        })
    }

    /// Removes Chord's mapping from every table when `owner` holds it or its holder
    /// has died. With no owner (the guardian), only a dead holder's mapping is removed.
    pub fn uninstall(
        &self,
        tables: &impl MappingTables,
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
            write_tables(tables, &tables.read()?, "unmap", |table| {
                let displaced = match table.id {
                    TableId::Keyboard(id) => state.keyboard_displaced.get(&id),
                    TableId::Default => None,
                };
                without_chord_mapping(&table.entries, displaced.unwrap_or(&state.displaced))
            })?;
            store.remove()?;
            Ok(true)
        })
    }
}

/// Every `UserKeyMapping` table, abstracted so the logic above is testable.
pub trait MappingTables {
    /// The default table followed by the table of each connected keyboard.
    fn read(&self) -> Result<Vec<Table>>;
    fn write(&self, id: TableId, entries: &[Mapping]) -> Result<()>;
}

fn write_tables(
    tables: &impl MappingTables,
    current: &[Table],
    action: &str,
    desired: impl Fn(&Table) -> Vec<Mapping>,
) -> Result<()> {
    let mut default_written: Option<Vec<Mapping>> = None;
    for table in current {
        let entries = desired(table);
        // Writing the default also sets every keyboard to that table. Restore any
        // differences from the snapshot, even if the keyboard was already correct.
        let live_entries = default_written.as_deref().unwrap_or(&table.entries);
        if entries.as_slice() != live_entries {
            tables
                .write(table.id, &entries)
                .with_context(|| format!("failed to {action} Caps on {}", table.name))?;
            if table.id == TableId::Default {
                default_written = Some(entries);
            }
        }
    }
    Ok(())
}

/// macOS's tables. The default is the property set on the HID event system itself;
/// writing it also replaces every keyboard's table. A keyboard's table is the same
/// property on its service.
pub struct SystemMappingTable;

// HID usage page and usage of a keyboard (`kHIDPage_GenericDesktop`, `kHIDUsage_GD_Keyboard`).
const KEYBOARD_USAGE: (u32, u32) = (0x01, 0x06);

impl SystemMappingTable {
    fn keyboards(client: &IOHIDEventSystemClient) -> Vec<CFRetained<IOHIDServiceClient>> {
        let Some(services) = client.services() else {
            return vec![];
        };
        // SAFETY: documented as an array of IOHIDServiceClient.
        let services = unsafe { CFRetained::cast_unchecked::<CFArray<CFType>>(services) };
        services
            .iter()
            .filter_map(|service| service.downcast::<IOHIDServiceClient>().ok())
            .filter(|service| service.conforms_to(KEYBOARD_USAGE.0, KEYBOARD_USAGE.1))
            .collect()
    }

    fn keyboard_id(service: &IOHIDServiceClient) -> Option<u64> {
        let id = service.registry_id();
        id.downcast_ref::<CFNumber>()
            .and_then(CFNumber::as_i64)
            .map(|id| id as u64)
    }
}

/// Decodes a `UserKeyMapping` value; an unset one is an empty table.
fn entries_from(value: Option<CFRetained<CFType>>) -> Result<Vec<Mapping>> {
    let Some(value) = value else {
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

/// Encodes a table as a `UserKeyMapping` value.
fn entries_value(entries: &[Mapping]) -> CFRetained<CFArray<CFDictionary<CFString, CFNumber>>> {
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
    CFArray::from_retained_objects(&dictionaries)
}

impl MappingTables for SystemMappingTable {
    fn read(&self) -> Result<Vec<Table>> {
        let key = CFString::from_static_str(USER_KEY_MAPPING);
        let client = IOHIDEventSystemClient::new_simple_client(None);
        let mut tables = vec![Table {
            id: TableId::Default,
            name: "the default table".into(),
            entries: entries_from(client.property(&key))?,
        }];
        for service in Self::keyboards(&client) {
            let Some(id) = Self::keyboard_id(&service) else {
                continue;
            };
            let product = service
                .property(&CFString::from_static_str("Product"))
                .and_then(|name| name.downcast_ref::<CFString>().map(ToString::to_string))
                .unwrap_or_else(|| "an unnamed keyboard".into());
            tables.push(Table {
                id: TableId::Keyboard(id),
                name: format!("{product} ({id:#x})"),
                entries: entries_from(service.property(&key))?,
            });
        }
        Ok(tables)
    }

    fn write(&self, id: TableId, entries: &[Mapping]) -> Result<()> {
        let key = CFString::from_static_str(USER_KEY_MAPPING);
        let value = entries_value(entries);
        let client = IOHIDEventSystemClient::new_simple_client(None);
        let applied = match id {
            // SAFETY: a property list value of the documented shape.
            TableId::Default => unsafe { client.set_property(&key, value.as_ref()) },
            TableId::Keyboard(id) => {
                let Some(service) = Self::keyboards(&client)
                    .into_iter()
                    .find(|service| Self::keyboard_id(service) == Some(id))
                else {
                    // Unplugged since it was read: nothing left to map.
                    return Ok(());
                };
                // SAFETY: a property list value of the documented shape.
                unsafe { service.set_property(&key, value.as_ref()) }
            }
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

/// Calls `changed` on the main thread each time any process writes the default
/// `UserKeyMapping` table, Chord included. macOS reports the write within milliseconds
/// (measured: 6 ms), so a mapping another tool removed is put back before the next Caps
/// press instead of being discovered by losing one. Only the first call registers.
///
/// A write to one keyboard's table is not reported here (seen 2026-11-26, UTM); that
/// one is found by the Caps press it lets through (`TapEvent::NativeCapsToggle`).
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
    const BUILT_IN: TableId = TableId::Keyboard(1);
    const EXTERNAL: TableId = TableId::Keyboard(2);

    /// Behaves like macOS: writing the default replaces every keyboard's table.
    struct Tables {
        tables: RefCell<Vec<Table>>,
        writes: std::cell::Cell<usize>,
    }

    impl Tables {
        fn new(keyboards: &[TableId]) -> Self {
            let ids = std::iter::once(TableId::Default).chain(keyboards.iter().copied());
            Self {
                tables: RefCell::new(
                    ids.map(|id| Table {
                        id,
                        name: format!("{id:?}"),
                        entries: vec![],
                    })
                    .collect(),
                ),
                writes: std::cell::Cell::new(0),
            }
        }

        fn entries(&self, id: TableId) -> Vec<Mapping> {
            let tables = self.tables.borrow();
            tables
                .iter()
                .find(|table| table.id == id)
                .unwrap()
                .entries
                .clone()
        }
    }

    impl MappingTables for Tables {
        fn read(&self) -> Result<Vec<Table>> {
            Ok(self.tables.borrow().clone())
        }
        fn write(&self, id: TableId, entries: &[Mapping]) -> Result<()> {
            for table in self.tables.borrow_mut().iter_mut() {
                if table.id == id || id == TableId::Default {
                    table.entries = entries.to_vec();
                }
            }
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
    fn only_a_table_with_chords_caps_entry_alone_is_chords() {
        assert!(is_chords(&[CHORD_MAPPING]));
        assert!(is_chords(&[OTHER, CHORD_MAPPING]));
        // Cleared by another tool, or never set.
        assert!(!is_chords(&[]));
        assert!(!is_chords(&[OTHER]));
        // Caps mapped elsewhere, with or without Chord's entry.
        let caps_elsewhere = (CAPS_USAGE, 0x7_0000_00E0);
        assert!(!is_chords(&[caps_elsewhere]));
        assert!(!is_chords(&[caps_elsewhere, CHORD_MAPPING]));
    }

    #[test]
    fn install_records_the_owner_before_mapping_and_uninstall_restores() {
        let tables = Tables::new(&[BUILT_IN]);
        tables.write(TableId::Default, &[OTHER, F18]).unwrap();
        let store = store();
        let me = ProcessIdentity::current().unwrap();
        store.install(&tables, me).unwrap();
        assert_eq!(tables.entries(TableId::Default), [OTHER, CHORD_MAPPING]);
        assert_eq!(tables.entries(BUILT_IN), [OTHER, CHORD_MAPPING]);
        let state = store.read().unwrap().unwrap();
        assert_eq!(
            (state.owner_pid, state.displaced.clone()),
            (me.pid, vec![F18])
        );
        // Re-installing keeps what was originally displaced and rewrites nothing.
        let writes = tables.writes.get();
        store.install(&tables, me).unwrap();
        assert_eq!(store.read().unwrap().unwrap().displaced, [F18]);
        assert_eq!(tables.writes.get(), writes);
        assert!(store.uninstall(&tables, Some(me)).unwrap());
        assert_eq!(tables.entries(TableId::Default), [OTHER, F18]);
        assert_eq!(tables.entries(BUILT_IN), [OTHER, F18]);
        assert_eq!(store.read().unwrap(), None);
        assert!(!store.uninstall(&tables, Some(me)).unwrap());
    }

    /// The outage of 2026-11-26: UTM cleared each keyboard's table and left the
    /// default alone, so the default still read as Chord's while Caps was plain Caps
    /// Lock on every keyboard.
    #[test]
    fn a_keyboard_cleared_behind_an_intact_default_is_seen_and_mapped_again() {
        let tables = Tables::new(&[BUILT_IN, EXTERNAL]);
        let store = store();
        let me = ProcessIdentity::current().unwrap();
        store.install(&tables, me).unwrap();
        assert_eq!(tables_without_chord_mapping(&tables).unwrap(), [""; 0]);

        tables.write(BUILT_IN, &[]).unwrap();
        assert_eq!(tables.entries(TableId::Default), [CHORD_MAPPING]);
        assert_eq!(
            tables_without_chord_mapping(&tables).unwrap(),
            [format!("{BUILT_IN:?}")]
        );

        // Only the keyboard that lost it is written.
        let writes = tables.writes.get();
        store.install(&tables, me).unwrap();
        assert_eq!(tables.writes.get(), writes + 1);
        assert_eq!(tables.entries(BUILT_IN), [CHORD_MAPPING]);
        assert_eq!(tables_without_chord_mapping(&tables).unwrap(), [""; 0]);
    }

    #[test]
    fn a_keyboard_keeps_its_own_entries_and_loses_only_chords_on_uninstall() {
        let tables = Tables::new(&[BUILT_IN, EXTERNAL]);
        tables.write(EXTERNAL, &[OTHER]).unwrap();
        let store = store();
        let me = ProcessIdentity::current().unwrap();
        store.install(&tables, me).unwrap();
        assert_eq!(tables.entries(BUILT_IN), [CHORD_MAPPING]);
        assert_eq!(tables.entries(EXTERNAL), [OTHER, CHORD_MAPPING]);
        assert!(store.uninstall(&tables, Some(me)).unwrap());
        assert_eq!(tables.entries(BUILT_IN), []);
        assert_eq!(tables.entries(EXTERNAL), [OTHER]);
        assert_eq!(tables.entries(TableId::Default), []);
    }

    #[test]
    fn changing_the_default_preserves_each_keyboards_own_mappings() {
        let tables = Tables::new(&[BUILT_IN, EXTERNAL]);
        let another_caps_mapping = (CAPS_USAGE, 0x7_0000_0029);
        tables.write(TableId::Default, &[F18]).unwrap();
        tables
            .write(BUILT_IN, &[OTHER, another_caps_mapping])
            .unwrap();
        tables.write(EXTERNAL, &[OTHER]).unwrap();
        let store = store();
        let me = ProcessIdentity::current().unwrap();
        store.install(&tables, me).unwrap();
        assert_eq!(tables.entries(BUILT_IN), [OTHER, CHORD_MAPPING]);
        assert_eq!(tables.entries(EXTERNAL), [OTHER, CHORD_MAPPING]);

        // Reset the default and restore a keyboard to a valid, custom table. It
        // still needs to be written after Chord repairs the default.
        tables.write(TableId::Default, &[]).unwrap();
        tables.write(BUILT_IN, &[OTHER, CHORD_MAPPING]).unwrap();
        tables.write(EXTERNAL, &[OTHER, CHORD_MAPPING]).unwrap();
        store.install(&tables, me).unwrap();
        assert_eq!(tables.entries(BUILT_IN), [OTHER, CHORD_MAPPING]);
        assert_eq!(tables.entries(EXTERNAL), [OTHER, CHORD_MAPPING]);

        assert!(store.uninstall(&tables, Some(me)).unwrap());
        assert_eq!(tables.entries(TableId::Default), [F18]);
        assert_eq!(tables.entries(BUILT_IN), [OTHER, another_caps_mapping]);
        assert_eq!(tables.entries(EXTERNAL), [OTHER]);
    }

    #[test]
    fn a_live_owner_keeps_the_mapping_while_a_dead_one_releases_it() {
        let tables = Tables::new(&[BUILT_IN]);
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
        store.install(&tables, me).unwrap();
        assert!(!store.uninstall(&tables, Some(other)).unwrap());
        assert!(!store.uninstall(&tables, None).unwrap());
        assert_eq!(tables.entries(BUILT_IN), [CHORD_MAPPING]);
        store.install(&tables, other).unwrap();
        assert!(!store.uninstall(&tables, Some(me)).unwrap());
        // Once its holder is gone, anyone releases it.
        let gone = ProcessIdentity {
            pid: u32::MAX >> 1,
            started: 0,
        };
        assert!(!gone.is_alive());
        store.install(&tables, gone).unwrap();
        assert!(store.uninstall(&tables, None).unwrap());
        assert_eq!(tables.entries(TableId::Default), []);
        assert_eq!(tables.entries(BUILT_IN), []);
        assert_eq!(store.read().unwrap(), None);
    }

    #[test]
    #[ignore = "reads this Mac's key mapping tables"]
    fn reads_the_default_table_and_each_keyboard() {
        let tables = SystemMappingTable.read().unwrap();
        for table in &tables {
            println!("{}: {:x?}", table.name, table.entries);
        }
        assert_eq!(tables[0].id, TableId::Default);
        println!(
            "without Chord's mapping: {:?}",
            tables_without_chord_mapping(&SystemMappingTable).unwrap()
        );
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

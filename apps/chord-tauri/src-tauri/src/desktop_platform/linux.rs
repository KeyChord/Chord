use super::{DesktopProcess, KeyFilter, Ready, executable_name, installed_app, running};
use crate::models::{Key, KeyEvent};
use anyhow::{Context, Result, ensure};
use evdev::{Device, EventType, InputEvent, KeyCode, uinput::VirtualDevice};
use keycode::{KeyMap, KeyMapping, KeyMappingCode};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
use x11rb::connection::{Connection, RequestConnection};
use x11rb::protocol::xkb::{ConnectionExt as _, Group};
use x11rb::protocol::xproto::{self, AtomEnum, ConnectionExt as _, ModMask};
use x11rb::protocol::xtest::ConnectionExt as _;
use x11rb::rust_connection::RustConnection;

const VIRTUAL_DEVICE: &str = "Chord virtual keyboard";
const CORE_KEYBOARD: u16 = 256;

fn connect() -> Result<(RustConnection, usize)> {
    Ok(x11rb::connect(None)?)
}

fn key_from_evdev(code: u16) -> Option<Key> {
    if code == 0 {
        return None;
    }
    Some(Key(KeyMap::from_key_mapping(KeyMapping::Evdev(code))
        .ok()?
        .code?))
}

fn keyboard(device: &Device) -> bool {
    device.name() != Some(VIRTUAL_DEVICE)
        && device
            .supported_keys()
            .is_some_and(|keys| keys.contains(KeyCode::KEY_A) && keys.contains(KeyCode::KEY_SPACE))
}

pub fn input_available() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_none()
        && std::env::var_os("DISPLAY").is_some()
        && std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/uinput")
            .is_ok()
        && evdev::enumerate().any(|(_, device)| keyboard(&device))
}

struct Keyboard {
    input: Device,
    output: VirtualDevice,
    grabbed: bool,
}

impl Keyboard {
    fn new(input: Device) -> Result<Self> {
        input.set_nonblocking(true)?;
        let output = VirtualDevice::builder()?
            .name(VIRTUAL_DEVICE)
            .with_keys(
                input
                    .supported_keys()
                    .context("keyboard has no key capabilities")?,
            )?
            .build()?;
        Ok(Self {
            input,
            output,
            grabbed: false,
        })
    }

    fn sync(&mut self, active: bool, filter: &mut KeyFilter) -> Result<()> {
        if !self.grabbed {
            // Let an in-progress keypress finish on the physical device before
            // redirecting its future events to the virtual keyboard.
            if active && self.input.get_key_state()?.iter().next().is_none() {
                self.input.grab()?;
                self.grabbed = true;
                while self.input.fetch_events().is_ok() {}
            } else {
                return Ok(());
            }
        }
        let events = match self.input.fetch_events() {
            Ok(events) => events,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        let mut forwarded = Vec::new();
        for event in events {
            if event.event_type() != EventType::KEY {
                continue;
            }
            let consumed = key_from_evdev(event.code()).is_some_and(|key| match event.value() {
                0 => filter(KeyEvent::Release(key)),
                1 | 2 => filter(KeyEvent::Press(key)),
                _ => false,
            });
            if !consumed {
                forwarded.push(InputEvent::new(
                    EventType::KEY.0,
                    event.code(),
                    event.value(),
                ));
            }
        }
        if !forwarded.is_empty() {
            self.output.emit(&forwarded)?;
        }
        Ok(())
    }
}

pub fn listen(
    stop: &AtomicBool,
    mut filter: Box<KeyFilter>,
    ready: Box<Ready>,
    owns_input: impl Fn() -> bool,
) -> Result<()> {
    let (connection, _) = connect()?;
    ensure!(
        connection
            .extension_information(x11rb::protocol::xtest::X11_EXTENSION_NAME)?
            .is_some(),
        "the X server does not support XTEST"
    );
    let mut devices = HashMap::<PathBuf, Keyboard>::new();
    let mut next_scan = Instant::now();
    let mut is_ready = false;
    while running(stop) {
        let active = owns_input();
        if !active {
            // Release physical devices while another Chord channel has the lease.
            devices.retain(|_, device| !device.grabbed);
        }
        if Instant::now() >= next_scan {
            for (path, input) in evdev::enumerate() {
                if keyboard(&input) && !devices.contains_key(&path) {
                    match Keyboard::new(input) {
                        Ok(device) => {
                            devices.insert(path, device);
                        }
                        Err(error) => log::warn!("Could not monitor {}: {error:#}", path.display()),
                    }
                }
            }
            ensure!(!devices.is_empty(), "no usable input keyboards");
            if !is_ready {
                ready();
                is_ready = true;
            }
            next_scan = Instant::now() + Duration::from_secs(1);
        }
        devices.retain(|path, device| match device.sync(active, filter.as_mut()) {
            Ok(()) => true,
            Err(error) => {
                log::warn!("Stopped monitoring {}: {error:#}", path.display());
                false
            }
        });
        std::thread::sleep(Duration::from_millis(2));
    }
    Ok(())
}

pub fn inject_key(key: Key, down: bool) -> Result<()> {
    let map = KeyMap::from(key.0);
    ensure!(
        map.evdev != 0 && map.xkb <= u16::from(u8::MAX),
        "unsupported X11 key {key}"
    );
    let (connection, screen) = connect()?;
    connection
        .xtest_fake_input(
            if down {
                xproto::KEY_PRESS_EVENT
            } else {
                xproto::KEY_RELEASE_EVENT
            },
            map.xkb as u8,
            0,
            connection.setup().roots[screen].root,
            0,
            0,
            0,
        )?
        .check()?;
    Ok(())
}

fn xkb_connect() -> Result<RustConnection> {
    let (connection, _) = connect()?;
    ensure!(
        connection.xkb_use_extension(1, 0)?.reply()?.supported,
        "XKB is unavailable"
    );
    Ok(connection)
}

pub fn caps_lock() -> Result<bool> {
    let locked = xkb_connect()?
        .xkb_get_state(CORE_KEYBOARD)?
        .reply()?
        .locked_mods;
    Ok(u16::from(locked & ModMask::LOCK) != 0)
}

pub fn set_caps_lock(enabled: bool) -> Result<()> {
    let connection = xkb_connect()?;
    connection
        .xkb_latch_lock_state(
            CORE_KEYBOARD,
            ModMask::LOCK,
            if enabled {
                ModMask::LOCK
            } else {
                ModMask::default()
            },
            false,
            Group::M1,
            ModMask::default(),
            false,
            0,
        )?
        .check()?;
    Ok(())
}

pub fn held_modifier_keys() -> Vec<Key> {
    let Ok((connection, _)) = connect() else {
        return Vec::new();
    };
    let Some(state) = connection
        .query_keymap()
        .ok()
        .and_then(|cookie| cookie.reply().ok())
    else {
        return Vec::new();
    };
    [
        KeyMappingCode::ShiftLeft,
        KeyMappingCode::ShiftRight,
        KeyMappingCode::ControlLeft,
        KeyMappingCode::ControlRight,
        KeyMappingCode::AltLeft,
        KeyMappingCode::AltRight,
        KeyMappingCode::MetaLeft,
        KeyMappingCode::MetaRight,
    ]
    .into_iter()
    .filter(|&code| {
        let xkb = usize::from(KeyMap::from(code).xkb);
        state
            .keys
            .get(xkb / 8)
            .is_some_and(|byte| byte & (1 << (xkb % 8)) != 0)
    })
    .map(Key)
    .collect()
}

fn active_window(connection: &RustConnection, screen: usize) -> Option<u32> {
    let name = connection
        .intern_atom(true, b"_NET_ACTIVE_WINDOW")
        .ok()?
        .reply()
        .ok()?
        .atom;
    let root = connection.setup().roots.get(screen)?.root;
    connection
        .get_property(false, root, name, AtomEnum::WINDOW, 0, 1)
        .ok()?
        .reply()
        .ok()?
        .value32()?
        .next()
        .filter(|&window| window != 0)
}

fn process_for_window(connection: &RustConnection, window: u32) -> Option<DesktopProcess> {
    let atom = connection
        .intern_atom(true, b"_NET_WM_PID")
        .ok()?
        .reply()
        .ok()?
        .atom;
    let pid = connection
        .get_property(false, window, atom, AtomEnum::CARDINAL, 0, 1)
        .ok()?
        .reply()
        .ok()?
        .value32()?
        .next()?;
    let window_class = connection
        .get_property(false, window, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 256)
        .ok()?
        .reply()
        .ok()?
        .value
        .split(|&byte| byte == 0)
        .filter(|value| !value.is_empty())
        .next_back()
        .map(|value| String::from_utf8_lossy(value).into_owned());
    let executable = std::fs::read_link(format!("/proc/{pid}/exe"))
        .ok()
        .and_then(executable_name)
        .or_else(|| window_class.clone())?;
    Some(DesktopProcess {
        pid,
        executable,
        window_class,
    })
}

pub fn foreground_app() -> Option<DesktopProcess> {
    let (connection, screen) = connect().ok()?;
    process_for_window(&connection, active_window(&connection, screen)?)
}

pub fn focused_window() -> Option<u32> {
    let (connection, screen) = connect().ok()?;
    active_window(&connection, screen)
}

fn client_windows(connection: &RustConnection, screen: usize) -> Result<Vec<u32>> {
    let root = connection.setup().roots[screen].root;
    for (name, reverse) in [
        (b"_NET_CLIENT_LIST_STACKING".as_slice(), true),
        (b"_NET_CLIENT_LIST".as_slice(), false),
    ] {
        let atom = connection.intern_atom(true, name)?.reply()?.atom;
        if atom == 0 {
            continue;
        }
        let reply = connection
            .get_property(false, root, atom, AtomEnum::WINDOW, 0, 4096)?
            .reply()?;
        if let Some(values) = reply.value32() {
            let mut windows: Vec<_> = values.collect();
            if reverse {
                windows.reverse();
            }
            return Ok(windows);
        }
    }
    Ok(Vec::new())
}

pub fn processes() -> Vec<DesktopProcess> {
    let Ok((connection, screen)) = connect() else {
        return Vec::new();
    };
    client_windows(&connection, screen)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|window| process_for_window(&connection, window))
        .collect()
}

fn describe_window(
    connection: &RustConnection,
    screen: usize,
    window: u32,
) -> Result<crate::native_input::NativeWindow> {
    let process =
        process_for_window(connection, window).context("target window has no application")?;
    let attributes = connection.get_window_attributes(window)?.reply()?;
    ensure!(
        attributes.map_state == xproto::MapState::VIEWABLE,
        "target window is hidden"
    );
    let geometry = connection.get_geometry(window)?.reply()?;
    let root = connection.setup().roots[screen].root;
    let position = connection
        .translate_coordinates(window, root, 0, 0)?
        .reply()?;
    let title = connection
        .get_property(false, window, AtomEnum::WM_NAME, AtomEnum::ANY, 0, 256)?
        .reply()?
        .value;
    Ok(crate::native_input::NativeWindow {
        window_id: window,
        process_id: process.pid as i32,
        bounds: crate::native_input::WindowBounds {
            x: f64::from(position.dst_x),
            y: f64::from(position.dst_y),
            width: f64::from(geometry.width),
            height: f64::from(geometry.height),
        },
        title: (!title.is_empty()).then(|| String::from_utf8_lossy(&title).into_owned()),
    })
}

pub fn window(id: u32) -> Result<crate::native_input::NativeWindow> {
    let (connection, screen) = connect()?;
    describe_window(&connection, screen, id)
}

pub fn windows(pids: &[i32]) -> Result<Vec<crate::native_input::NativeWindow>> {
    let (connection, screen) = connect()?;
    Ok(client_windows(&connection, screen)?
        .into_iter()
        .filter_map(|id| {
            let window = describe_window(&connection, screen, id).ok()?;
            pids.contains(&window.process_id).then_some(window)
        })
        .collect())
}

pub fn screen_bounds() -> Result<crate::native_input::WindowBounds> {
    let (connection, screen) = connect()?;
    let screen = &connection.setup().roots[screen];
    Ok(crate::native_input::WindowBounds {
        x: 0.0,
        y: 0.0,
        width: f64::from(screen.width_in_pixels),
        height: f64::from(screen.height_in_pixels),
    })
}

pub fn move_mouse(x: f64, y: f64) -> Result<()> {
    let bounds = screen_bounds()?;
    ensure!(
        x >= 0.0
            && y >= 0.0
            && x < bounds.width
            && y < bounds.height
            && x <= f64::from(i16::MAX)
            && y <= f64::from(i16::MAX),
        "point is outside the desktop"
    );
    let (connection, screen) = connect()?;
    connection
        .xtest_fake_input(
            xproto::MOTION_NOTIFY_EVENT,
            0,
            0,
            connection.setup().roots[screen].root,
            x.round() as i16,
            y.round() as i16,
            0,
        )?
        .check()?;
    Ok(())
}

pub fn pointer_targets_window(id: u32) -> Result<bool> {
    let (connection, screen) = connect()?;
    let root = connection.setup().roots[screen].root;
    let mut target = id;
    for _ in 0..16 {
        let parent = connection.query_tree(target)?.reply()?.parent;
        if parent == root {
            return Ok(connection.query_pointer(root)?.reply()?.child == target);
        }
        if parent == 0 {
            break;
        }
        target = parent;
    }
    Ok(false)
}

pub fn mouse_button(button: crate::native_input::MouseButton, down: bool) -> Result<()> {
    use crate::native_input::MouseButton::*;
    let detail = match button {
        Left => 1,
        Middle => 2,
        Right => 3,
    };
    let (connection, screen) = connect()?;
    connection
        .xtest_fake_input(
            if down {
                xproto::BUTTON_PRESS_EVENT
            } else {
                xproto::BUTTON_RELEASE_EVENT
            },
            detail,
            0,
            connection.setup().roots[screen].root,
            0,
            0,
            0,
        )?
        .check()?;
    Ok(())
}

fn desktop_entries() -> BTreeMap<String, String> {
    // Desktop file IDs are the stable launch target used by gtk-launch and XDG.
    let home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));
    let data_dirs =
        std::env::var_os("XDG_DATA_DIRS").unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    let mut apps = BTreeMap::new();
    for directory in home.into_iter().chain(std::env::split_paths(&data_dirs)) {
        let directory = directory.join("applications");
        for entry in walkdir::WalkDir::new(&directory)
            .max_depth(3)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !entry.file_type().is_file()
                || path.extension().is_none_or(|suffix| suffix != "desktop")
            {
                continue;
            }
            let Ok(contents) = std::fs::read_to_string(path) else {
                continue;
            };
            let Some(section) = contents.split_once("[Desktop Entry]").map(|(_, value)| {
                value
                    .lines()
                    .take_while(|line| !line.starts_with('['))
                    .collect::<Vec<_>>()
            }) else {
                continue;
            };
            if section
                .iter()
                .any(|line| matches!(*line, "NoDisplay=true" | "Hidden=true" | "Type=Link"))
            {
                continue;
            }
            let Some(name) = section.iter().find_map(|line| line.strip_prefix("Name=")) else {
                continue;
            };
            let Ok(relative) = path.strip_prefix(&directory) else {
                continue;
            };
            let desktop = relative
                .with_extension("")
                .to_string_lossy()
                .replace('/', "-");
            if desktop.starts_with('-')
                || !desktop
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b".-_".contains(&c))
            {
                continue;
            }
            apps.entry(format!("desktop.{desktop}"))
                .or_insert_with(|| name.to_owned());
        }
    }
    apps
}

pub fn installed_apps() -> Vec<crate::api::InstalledApp> {
    desktop_entries()
        .into_iter()
        .map(|(id, name)| installed_app(id, name))
        .collect()
}

pub fn launch_command(id: &str) -> Result<String> {
    let desktop = id
        .strip_prefix("desktop.")
        .context("Invalid Linux desktop app ID")?;
    ensure!(
        desktop_entries().contains_key(id),
        "Desktop app is unavailable"
    );
    Ok(format!("gtk-launch -- {desktop}"))
}

pub fn relaunch_app(_app_id: &str) -> Result<()> {
    anyhow::bail!("This action needs a desktop-specific relaunch command")
}

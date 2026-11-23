use super::{
    DesktopProcess, KeyFilter, Ready, executable_id, executable_name, installed_app, running,
};
use crate::models::{Key, KeyEvent};
use anyhow::{Result, ensure};
use keycode::{KeyMap, KeyMapping, KeyMappingCode};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::ptr::null_mut;
use std::sync::atomic::AtomicBool;
use windows_sys::Win32::Foundation::{
    CloseHandle, HWND, INVALID_HANDLE_VALUE, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows_sys::Win32::System::{
    Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    },
    LibraryLoader::GetModuleHandleW,
    Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW},
};
use windows_sys::Win32::UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*};

thread_local! {
    static FILTER: RefCell<Option<Box<KeyFilter>>> = RefCell::new(None);
}

const SYNTHETIC_INPUT: usize = 0x4348_4f52;

fn keyboard_event(raw: &KBDLLHOOKSTRUCT) -> Option<KeyEvent> {
    if raw.flags & LLKHF_INJECTED != 0 && raw.dwExtraInfo == SYNTHETIC_INPUT {
        return None;
    }
    let scan = u16::try_from(raw.scanCode).ok()?;
    if scan == 0 {
        return None;
    }
    let scan = scan
        | if raw.flags & LLKHF_EXTENDED != 0 {
            0xe000
        } else {
            0
        };
    let key = Key(KeyMap::from_key_mapping(KeyMapping::Win(scan)).ok()?.code?);
    Some(if raw.flags & LLKHF_UP != 0 {
        KeyEvent::Release(key)
    } else {
        KeyEvent::Press(key)
    })
}

unsafe extern "system" fn keyboard_hook(code: i32, param: WPARAM, data: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && data != 0 {
        // The pointer is valid until this low-level hook call returns.
        let raw = unsafe { &*(data as *const KBDLLHOOKSTRUCT) };
        if let Some(event) = keyboard_event(raw) {
            let consumed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                FILTER.with(|slot| {
                    slot.borrow_mut()
                        .as_mut()
                        .is_some_and(|filter| filter(event))
                })
            }))
            .unwrap_or(false);
            if consumed {
                return 1;
            }
        }
    }
    unsafe { CallNextHookEx(null_mut(), code, param, data) }
}

pub fn input_available() -> bool {
    true
}

pub fn listen(
    stop: &AtomicBool,
    filter: Box<KeyFilter>,
    ready: Box<Ready>,
    _owns_input: impl Fn() -> bool,
) -> Result<()> {
    FILTER.with(|slot| slot.replace(Some(filter)));
    let hook = unsafe {
        SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(keyboard_hook),
            GetModuleHandleW(std::ptr::null()),
            0,
        )
    };
    if hook.is_null() {
        FILTER.with(|slot| slot.replace(None));
        return Err(std::io::Error::last_os_error().into());
    }
    ready();
    let mut message = MSG::default();
    let result = loop {
        if !running(stop) {
            break Ok(());
        }
        match unsafe { GetMessageW(&mut message, null_mut(), 0, 0) } {
            -1 => break Err(std::io::Error::last_os_error().into()),
            0 => break Ok(()),
            _ => unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            },
        }
    };
    unsafe {
        UnhookWindowsHookEx(hook);
    }
    FILTER.with(|slot| slot.replace(None));
    result
}

pub fn inject_key(key: Key, down: bool) -> Result<()> {
    let scan = KeyMap::from(key.0).win;
    ensure!(scan != 0, "unsupported Windows key {key}");
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: 0,
                wScan: scan & 0xff,
                dwFlags: KEYEVENTF_SCANCODE
                    | if scan >> 8 == 0xe0 {
                        KEYEVENTF_EXTENDEDKEY
                    } else {
                        0
                    }
                    | if down { 0 } else { KEYEVENTF_KEYUP },
                time: 0,
                dwExtraInfo: SYNTHETIC_INPUT,
            },
        },
    };
    if unsafe { SendInput(1, &input, std::mem::size_of::<INPUT>() as i32) } != 1 {
        return Err(std::io::Error::last_os_error()).map_err(Into::into);
    }
    Ok(())
}

pub fn caps_lock() -> Result<bool> {
    Ok(unsafe { GetKeyState(i32::from(VK_CAPITAL)) } & 1 != 0)
}

pub fn set_caps_lock(enabled: bool) -> Result<()> {
    if caps_lock()? != enabled {
        let key = Key(KeyMappingCode::CapsLock);
        inject_key(key, true)?;
        inject_key(key, false)?;
    }
    Ok(())
}

pub fn held_modifier_keys() -> Vec<Key> {
    [
        (VK_LSHIFT, KeyMappingCode::ShiftLeft),
        (VK_RSHIFT, KeyMappingCode::ShiftRight),
        (VK_LCONTROL, KeyMappingCode::ControlLeft),
        (VK_RCONTROL, KeyMappingCode::ControlRight),
        (VK_LMENU, KeyMappingCode::AltLeft),
        (VK_RMENU, KeyMappingCode::AltRight),
        (VK_LWIN, KeyMappingCode::MetaLeft),
        (VK_RWIN, KeyMappingCode::MetaRight),
    ]
    .into_iter()
    .filter(|&(vk, _)| unsafe { GetAsyncKeyState(i32::from(vk)) } < 0)
    .map(|(_, code)| Key(code))
    .collect()
}

fn process_path(pid: u32) -> Option<PathBuf> {
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if process.is_null() {
        return None;
    }
    let mut path = vec![0u16; 32768];
    let mut length = path.len() as u32;
    let success = unsafe {
        let success = QueryFullProcessImageNameW(process, 0, path.as_mut_ptr(), &mut length);
        CloseHandle(process);
        success
    };
    if success == 0 {
        return None;
    }
    Some(PathBuf::from(String::from_utf16_lossy(
        &path[..length as usize],
    )))
}

fn process_executable(pid: u32) -> Option<String> {
    executable_name(process_path(pid)?)
}

fn foreground() -> HWND {
    unsafe { GetForegroundWindow() }
}

fn window_class(window: HWND) -> Option<String> {
    let mut name = [0u16; 256];
    let length = unsafe { GetClassNameW(window, name.as_mut_ptr(), name.len() as i32) };
    (length > 0).then(|| String::from_utf16_lossy(&name[..length as usize]))
}

pub fn foreground_app() -> Option<DesktopProcess> {
    let window = foreground();
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(window, &mut pid);
    }
    let executable = process_executable(pid)?;
    Some(DesktopProcess {
        pid,
        executable,
        window_class: window_class(window),
    })
}

pub fn focused_window() -> Option<u32> {
    let window = foreground();
    (!window.is_null()).then_some(window as usize as u32)
}

pub fn processes() -> Vec<DesktopProcess> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Vec::new();
    }
    let mut output = Vec::new();
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut more = unsafe { Process32FirstW(snapshot, &mut entry) };
    while more != 0 {
        let end = entry
            .szExeFile
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(entry.szExeFile.len());
        output.push(DesktopProcess {
            pid: entry.th32ProcessID,
            executable: String::from_utf16_lossy(&entry.szExeFile[..end]),
            window_class: None,
        });
        more = unsafe { Process32NextW(snapshot, &mut entry) };
    }
    unsafe {
        CloseHandle(snapshot);
    }
    output
}

pub fn installed_apps() -> Vec<crate::api::InstalledApp> {
    unsafe extern "system" fn collect(window: HWND, data: LPARAM) -> i32 {
        if unsafe { IsWindowVisible(window) } != 0
            && unsafe { GetWindow(window, GW_OWNER) }.is_null()
        {
            let mut pid = 0;
            unsafe {
                GetWindowThreadProcessId(window, &mut pid);
            }
            if let Some(name) = process_executable(pid) {
                let apps = unsafe { &mut *(data as *mut BTreeMap<String, String>) };
                apps.entry(executable_id(&name)).or_insert(name);
            }
        }
        1
    }
    let mut apps = BTreeMap::new();
    unsafe {
        EnumWindows(Some(collect), &mut apps as *mut _ as LPARAM);
    }
    apps.into_iter()
        .map(|(id, name)| installed_app(id, name))
        .collect()
}

fn from_window_id(id: u32) -> HWND {
    id as i32 as isize as HWND
}

fn describe_window(window: HWND) -> Option<crate::native_input::NativeWindow> {
    if unsafe { IsWindowVisible(window) } == 0 || unsafe { IsIconic(window) } != 0 {
        return None;
    }
    let mut rect = RECT::default();
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(window, &mut pid);
        if GetWindowRect(window, &mut rect) == 0 {
            return None;
        }
    }
    let mut title = [0u16; 512];
    let length = unsafe { GetWindowTextW(window, title.as_mut_ptr(), title.len() as i32) };
    Some(crate::native_input::NativeWindow {
        window_id: window as usize as u32,
        process_id: pid as i32,
        bounds: crate::native_input::WindowBounds {
            x: f64::from(rect.left),
            y: f64::from(rect.top),
            width: f64::from(rect.right - rect.left),
            height: f64::from(rect.bottom - rect.top),
        },
        title: (length > 0).then(|| String::from_utf16_lossy(&title[..length as usize])),
    })
}

pub fn window(id: u32) -> Result<crate::native_input::NativeWindow> {
    describe_window(from_window_id(id))
        .ok_or_else(|| anyhow::anyhow!("target window is unavailable"))
}

pub fn windows(pids: &[i32]) -> Result<Vec<crate::native_input::NativeWindow>> {
    type State<'a> = (&'a [i32], Vec<crate::native_input::NativeWindow>);
    unsafe extern "system" fn collect(window: HWND, data: LPARAM) -> i32 {
        let (pids, windows) = unsafe { &mut *(data as *mut State<'_>) };
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(window, &mut pid);
        }
        if pids.contains(&(pid as i32))
            && let Some(window) = describe_window(window)
        {
            windows.push(window);
        }
        1
    }
    let mut state = (pids, Vec::new());
    if unsafe { EnumWindows(Some(collect), &mut state as *mut _ as LPARAM) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(state.1)
}

pub fn screen_bounds() -> Result<crate::native_input::WindowBounds> {
    let metric = |index| f64::from(unsafe { GetSystemMetrics(index) });
    Ok(crate::native_input::WindowBounds {
        x: metric(SM_XVIRTUALSCREEN),
        y: metric(SM_YVIRTUALSCREEN),
        width: metric(SM_CXVIRTUALSCREEN),
        height: metric(SM_CYVIRTUALSCREEN),
    })
}

pub fn move_mouse(x: f64, y: f64) -> Result<()> {
    let bounds = screen_bounds()?;
    ensure!(
        x >= bounds.x
            && y >= bounds.y
            && x < bounds.x + bounds.width
            && y < bounds.y + bounds.height,
        "point is outside the desktop"
    );
    if unsafe { SetCursorPos(x.round() as i32, y.round() as i32) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

pub fn pointer_targets_window(id: u32) -> Result<bool> {
    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let target = unsafe { WindowFromPoint(point) };
    Ok(!target.is_null() && unsafe { GetAncestor(target, GA_ROOT) } == from_window_id(id))
}

pub fn mouse_button(button: crate::native_input::MouseButton, down: bool) -> Result<()> {
    use crate::native_input::MouseButton::*;
    let flag = match (button, down) {
        (Left, true) => MOUSEEVENTF_LEFTDOWN,
        (Left, false) => MOUSEEVENTF_LEFTUP,
        (Right, true) => MOUSEEVENTF_RIGHTDOWN,
        (Right, false) => MOUSEEVENTF_RIGHTUP,
        (Middle, true) => MOUSEEVENTF_MIDDLEDOWN,
        (Middle, false) => MOUSEEVENTF_MIDDLEUP,
    };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dwFlags: flag,
                ..Default::default()
            },
        },
    };
    if unsafe { SendInput(1, &input, std::mem::size_of::<INPUT>() as i32) } != 1 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

pub fn launch_command(id: &str) -> Result<String> {
    let path = processes()
        .into_iter()
        .filter(|process| executable_id(&process.executable) == id)
        .find_map(|process| process_path(process.pid))
        .ok_or_else(|| anyhow::anyhow!("Application is unavailable; open it first"))?;
    let path = path.to_string_lossy();
    ensure!(
        !path.contains('%'),
        "The application path contains an unsupported character"
    );
    Ok(format!(r#"start "" "{path}""#))
}

pub fn relaunch_app(_app_id: &str) -> Result<()> {
    anyhow::bail!("This action needs a desktop-specific relaunch command")
}

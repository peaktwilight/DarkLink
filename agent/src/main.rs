mod file_handling;

use file_handling::{download::download_file, upload::upload_file, test_server};

#[tokio::main]
async fn main() {
    println!("MicroC2 Agent starting...");

    // Start test server in background
    tokio::spawn(test_server::run_test_server());

    // Test configuration
    let proxy_addr = "127.0.0.1:1080";
    let target_addr = "127.0.0.1:8080";

    if let Err(e) = run_file_operations(proxy_addr, target_addr).await {
        eprintln!("Error during file operations: {}", e);
    }
}

async fn run_file_operations(proxy_addr: &str, target_addr: &str) -> std::io::Result<()> {
    // Upload test.txt to local test server
    upload_file(
        "src/test.txt",
        "/upload",
        proxy_addr,
        target_addr
    ).await?;

    println!("Test upload completed. Check received_test.txt for the uploaded content.");
    Ok(())
}

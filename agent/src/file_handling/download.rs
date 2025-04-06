use std::io::{self, Read};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub async fn download_file(
    local_path: &str,
    remote_path: &str,
    _proxy_addr: &str,
    target_addr: &str,
) -> io::Result<()> {
    // Ensure remote_path starts with '/'
    let path = if !remote_path.starts_with('/') {
        format!("/{}", remote_path)
    } else {
        remote_path.to_string()
    };

    // Construct proper URL with filename
    let url = format!("http://{}/download/{}", target_addr, 
                     path.trim_start_matches('/').trim_start_matches("download/"));

    println!("Downloading from: {}", url);

    // Get response with better error handling
    let response = ureq::get(&url)
        .call()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Download failed: {}", e)))?;

    if response.status() != 200 {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Server returned status: {}", response.status())
        ));
    }

    // Read response body
    let mut bytes = Vec::new();
    response.into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to read response: {}", e)))?;

    if bytes.is_empty() {
        return Err(io::Error::new(io::ErrorKind::Other, "Received empty response"));
    }

    // Write to file
    let mut file = File::create(local_path).await?;
    file.write_all(&bytes).await?;
    
    println!("Successfully downloaded {} bytes to {}", bytes.len(), local_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[tokio::test]
    async fn test_download_functionality() {
        let test_file = "test_download.txt";
        let download_path = PathBuf::from("downloaded_file.txt");
        
        // Test download
        let result = download_file(
            download_path.to_str().unwrap(),
            test_file,
            "",
            "127.0.0.1:8080"   // Test server address
        ).await;
        
        assert!(result.is_ok());
        
        // Cleanup
        if download_path.exists() {
            std::fs::remove_file(download_path).unwrap();
        }
    }
}

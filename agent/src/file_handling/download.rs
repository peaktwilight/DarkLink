use std::io;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub async fn download_file(
    local_path: &str,
    _remote_path: &str,
    _proxy_addr: &str,
    target_addr: &str,
) -> io::Result<()> {
    let url = format!("http://{}{}", target_addr, _remote_path);
    let response = ureq::get(&url).call().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let mut bytes = Vec::new();
    response.into_reader().read_to_end(&mut bytes)?;
    
    let mut file = File::create(local_path).await?;
    file.write_all(&bytes).await?;
    
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

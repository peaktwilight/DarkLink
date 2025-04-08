use std::fs;
use std::io;

pub async fn download_file(url: &str, path: &str) -> io::Result<()> {
    let response = reqwest::get(url)
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        
    let bytes = response.bytes()
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        
    fs::write(path, bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[tokio::test]
    async fn test_download_functionality() {
        let test_file = "http://127.0.0.1:8080/download/test_download.txt";
        let download_path = PathBuf::from("downloaded_file.txt");
        
        // Test download
        let result = download_file(
            test_file,
            download_path.to_str().unwrap()
        ).await;
        
        assert!(result.is_ok());
        
        // Cleanup
        if download_path.exists() {
            std::fs::remove_file(download_path).unwrap();
        }
    }
}

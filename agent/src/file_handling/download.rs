use std::error::Error;
use hyper::{Client, Body};
use bytes::Bytes;

pub async fn download_file(url: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let client = Client::new();
    
    let resp = client.get(url.parse()?).await?;
    if !resp.status().is_success() {
        return Err("Download failed".into());
    }
    
    let body = hyper::body::to_bytes(resp.into_body()).await?;
    Ok(body.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    
    #[tokio::test]
    async fn test_download_functionality() {
        let test_file = "http://127.0.0.1:8080/download/test_download.txt";
        let download_path = PathBuf::from("downloaded_file.txt");
        
        // Test download
        let result = download_file(test_file).await;
        
        assert!(result.is_ok());
        
        // Write downloaded content to file
        if let Ok(content) = result {
            fs::write(&download_path, content).unwrap();
        }
        
        // Cleanup
        if download_path.exists() {
            fs::remove_file(download_path).unwrap();
        }
    }
}

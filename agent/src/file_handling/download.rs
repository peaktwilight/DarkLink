use std::error::Error;
use hyper::Client;
use hyper::body::HttpBody as _;


/// Download a file by streaming chunks directly to disk at `dest_path`, avoiding full in-memory buffering
pub async fn download_file(url: &str, dest_path: &std::path::Path) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let mut resp = client.get(url.parse()?).await?;
    if !resp.status().is_success() {
        return Err("Download failed".into());
    }
    // Create file asynchronously
    let mut file = tokio::fs::File::create(dest_path).await?;
    // Stream body chunks to file
    while let Some(chunk) = resp.body_mut().data().await {
        let bytes = chunk?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &bytes).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    
    #[tokio::test]
    async fn test_download_functionality() -> Result<(), Box<dyn std::error::Error>> {
        let test_file = "http://127.0.0.1:8080/download/test_download.txt";
        let download_path = PathBuf::from("downloaded_file.txt");
        
        // Test download and write to file
        download_file(test_file, &download_path).await?;
        
        // Cleanup
        if download_path.exists() {
            fs::remove_file(&download_path)?;
        }
        Ok(())
    }
}

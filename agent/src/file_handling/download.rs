use std::fs::File;
use std::io::{self, Write};
use tokio::io::AsyncReadExt;
use tokio_socks::tcp::Socks5Stream;

const BUFFER_SIZE: usize = 8192;

pub async fn download_file(
    remote_path: &str,
    local_path: &str, 
    proxy_addr: &str,
    target_addr: &str
) -> io::Result<()> {
    let mut stream = Socks5Stream::connect(proxy_addr, target_addr)
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let mut file = File::create(local_path)?;
    let mut buffer = vec![0u8; BUFFER_SIZE];

    loop {
        let bytes_read = stream.read(&mut buffer).await?;
        if bytes_read == 0 { break; }
        file.write_all(&buffer[..bytes_read])?;
    }

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
            test_file,
            download_path.to_str().unwrap(),
            "127.0.0.1:1080",  // SOCKS5 proxy address
            "127.0.0.1:8080"   // Test server address
        ).await;
        
        assert!(result.is_ok());
        
        // Cleanup
        if download_path.exists() {
            std::fs::remove_file(download_path).unwrap();
        }
    }
}

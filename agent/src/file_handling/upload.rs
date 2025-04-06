use std::io;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub async fn upload_file(
    local_path: &str,
    remote_path: &str,
    _proxy_addr: &str,
    target_addr: &str,
) -> io::Result<()> {
    let mut file = File::open(local_path).await?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).await?;
    
    let url = format!("http://{}{}", target_addr, remote_path);
    ureq::post(&url)
        .send_bytes(&contents)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    
    Ok(())
}

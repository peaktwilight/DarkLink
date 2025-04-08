use std::io;
use std::path::Path;
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
    let filename = Path::new(local_path).file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid filename"))?
        .to_string_lossy();

    ureq::post(&url)
        .set("X-Filename", &filename)
        .send_bytes(&contents)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    
    Ok(())
}

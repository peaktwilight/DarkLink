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

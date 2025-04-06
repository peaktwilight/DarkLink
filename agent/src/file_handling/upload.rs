use std::fs::File;
use std::io::{self, Read};
use tokio::io::AsyncWriteExt;
use tokio_socks::tcp::Socks5Stream;

const BUFFER_SIZE: usize = 8192;

pub async fn upload_file(
    local_path: &str,
    remote_path: &str,
    proxy_addr: &str,
    target_addr: &str
) -> io::Result<()> {
    let mut stream = Socks5Stream::connect(proxy_addr, target_addr)
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let mut file = File::open(local_path)?;
    let mut buffer = vec![0u8; BUFFER_SIZE];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 { break; }
        stream.write_all(&buffer[..bytes_read]).await?;
    }
    
    stream.flush().await?;
    Ok(())
}

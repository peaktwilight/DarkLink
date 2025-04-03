pub async fn handle_socks5(stream: &mut TcpStream) -> Result<()> {
    // Basic SOCKS5 handshake
    let mut auth = [0; 2];
    stream.read_exact(&mut auth).await?;
    
    // No authentication required
    stream.write_all(&[0x05, 0x00]).await?;
    
    // Handle command
    let mut request = [0; 4];
    stream.read_exact(&mut request).await?;
    
    if request[1] == 0x01 { // CONNECT
        let addr = read_address(stream).await?;
        proxy_connection(stream, addr).await
    } else {
        Err("Unsupported command".into())
    }
}
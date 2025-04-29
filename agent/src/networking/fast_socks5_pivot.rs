use socks5_proxy::server::Server;
use log::{info, error};

pub async fn start_socks5_server(listen_addr: &str, listen_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", listen_addr, listen_port);
    info!("[SOCKS5] Listening on {}", addr);

    let mut server = Server::bind(addr).await?;
    if let Err(e) = server.run().await {
        error!("[SOCKS5] Server error: {:?}", e);
    }
    Ok(())
}
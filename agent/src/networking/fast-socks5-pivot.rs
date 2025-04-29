use fast_socks5::server::{Config, Socks5Server};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use log::{info, error, debug};

pub async fn start_socks5_server(listen_addr: &str, listen_port: u16) -> anyhow::Result<()> {
    let addr = format!("{}:{}", listen_addr, listen_port);
    let socket_addr: SocketAddr = addr.parse()?;
    let listener = TcpListener::bind(socket_addr).await?;
    let config = Config::default();

    info!("[SOCKS5] Listening on {}", addr);
    debug!("[SOCKS5] Binding TcpListener to {}", addr);

    loop {
        debug!("[SOCKS5] Waiting for incoming SOCKS5 connection...");
        let (stream, peer_addr) = match listener.accept().await {
            Ok(pair) => {
                debug!("[SOCKS5] Accepted connection from {}", pair.1);
                pair
            },
            Err(e) => {
                error!("[SOCKS5] Failed to accept connection: {}", e);
                continue;
            }
        };
        let config = config.clone();
        tokio::spawn(async move {
            debug!("[SOCKS5] Spawning handler for {}", peer_addr);
            if let Err(e) = Socks5Server::new(config).serve(stream).await {
                error!("[SOCKS5] Error serving {}: {:?}", peer_addr, e);
            } else {
                debug!("[SOCKS5] Connection with {} handled successfully", peer_addr);
            }
        });
    }
}
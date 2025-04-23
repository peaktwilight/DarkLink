use tokio::net::{TcpStream, TcpListener};
use tokio_socks::tcp::Socks5Stream;
use std::net::SocketAddr;
use std::time::Duration;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::io;

// SOCKS5 Protocol Constants
pub const SOCKS5_VERSION: u8 = 0x05;

// Authentication Methods
pub const AUTH_NONE: u8 = 0x00;
pub const AUTH_GSSAPI: u8 = 0x01;
pub const AUTH_PASSWORD: u8 = 0x02;
pub const AUTH_NO_ACCEPT: u8 = 0xFF;

// Commands
pub const CMD_CONNECT: u8 = 0x01;
pub const CMD_BIND: u8 = 0x02;
pub const CMD_UDP_ASSOC: u8 = 0x03;

// Address Types
pub const ADDR_TYPE_IPV4: u8 = 0x01;
pub const ADDR_TYPE_DOMAIN: u8 = 0x03;
pub const ADDR_TYPE_IPV6: u8 = 0x04;

// Reply Codes
pub const REP_SUCCESS: u8 = 0x00;
pub const REP_SERVER_FAILURE: u8 = 0x01;
pub const REP_NOT_ALLOWED: u8 = 0x02;
pub const REP_NETWORK_UNREACH: u8 = 0x03;
pub const REP_HOST_UNREACH: u8 = 0x04;
pub const REP_CONN_REFUSED: u8 = 0x05;
pub const REP_TTL_EXPIRED: u8 = 0x06;
pub const REP_CMD_NOT_SUPPORTED: u8 = 0x07;
pub const REP_ADDR_NOT_SUPPORTED: u8 = 0x08;

#[derive(Error, Debug)]
pub enum Socks5Error {
    #[error("Connection timeout")]
    Timeout,
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Authentication failed")]
    AuthenticationFailed,
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    #[error("Proxy error: {0}")]
    ProxyError(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct Socks5Config {
    pub listen_addr: String,
    pub listen_port: u16,
    pub require_auth: bool,
    pub username: Option<String>,
    pub password: Option<String>,
    pub timeout_seconds: u32,
    pub allowed_ips: Vec<String>,
    pub disallowed_ports: Vec<u16>,
    pub connection_timeout: Duration,
    pub max_connections: usize,
}

impl Default for Socks5Config {
    fn default() -> Self {
        Self {
            listen_addr: String::from("127.0.0.1"),
            listen_port: 1080,
            require_auth: false,
            username: None,
            password: None,
            timeout_seconds: 30,
            allowed_ips: Vec::new(),
            disallowed_ports: Vec::new(),
            connection_timeout: Duration::from_secs(30),
            max_connections: 100,
        }
    }
}

#[derive(Clone)]
pub struct Socks5Client {
    proxy_addr: String,
    proxy_port: u16,
    username: Option<String>,
    password: Option<String>,
    timeout: Duration,
    connection_pool: Arc<Mutex<HashMap<String, Vec<TcpStream>>>>,
}

impl Socks5Client {
    pub fn new(proxy_addr: String, proxy_port: u16) -> Self {
        Self {
            proxy_addr,
            proxy_port,
            username: None,
            password: None,
            timeout: Duration::from_secs(30),
            connection_pool: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_auth(mut self, username: String, password: String) -> Self {
        self.username = Some(username);
        self.password = Some(password);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    async fn get_pooled_connection(&self, target: &str) -> Option<TcpStream> {
        let mut pool = self.connection_pool.lock().await;
        if let Some(connections) = pool.get_mut(target) {
            connections.pop()
        } else {
            None
        }
    }

    async fn store_connection(&self, target: String, conn: TcpStream) {
        let mut pool = self.connection_pool.lock().await;
        let connections = pool.entry(target).or_insert_with(Vec::new);
        if connections.len() < 10 { // Max 10 connections per target
            connections.push(conn);
        }
    }

    pub async fn connect_to(&self, target_addr: String, target_port: u16) -> Result<TcpStream, Socks5Error> {
        // Try to get a pooled connection first
        let target_key = format!("{}:{}", target_addr, target_port);
        if let Some(conn) = self.get_pooled_connection(&target_key).await {
            return Ok(conn);
        }

        let proxy_addr = format!("{}:{}", self.proxy_addr, self.proxy_port);
        let addr = proxy_addr.parse::<SocketAddr>()
            .map_err(|e| Socks5Error::InvalidAddress(e.to_string()))?;
        
        let target = format!("{}:{}", target_addr, target_port);

        let stream = match (&self.username, &self.password) {
            (Some(user), Some(pass)) => {
                tokio::time::timeout(
                    self.timeout,
                    Socks5Stream::connect_with_password(
                        addr,
                        target,
                        user,
                        pass,
                    )
                ).await
                .map_err(|_| Socks5Error::Timeout)?
                .map_err(|e| Socks5Error::ConnectionFailed(e.to_string()))?
            },
            _ => {
                tokio::time::timeout(
                    self.timeout,
                    Socks5Stream::connect(
                        addr,
                        target,
                    )
                ).await
                .map_err(|_| Socks5Error::Timeout)?
                .map_err(|e| Socks5Error::ConnectionFailed(e.to_string()))?
            }
        };

        let tcp_stream = stream.into_inner();

        Ok(tcp_stream)
    }

    pub async fn connect_with_retries(&self, target_addr: String, target_port: u16, retries: u32) -> Result<TcpStream, Socks5Error> {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts < retries {
            match self.connect_to(target_addr.clone(), target_port).await {
                Ok(stream) => return Ok(stream),
                Err(e) => {
                    last_error = Some(e);
                    attempts += 1;
                    if attempts < retries {
                        tokio::time::sleep(Duration::from_secs(1 << attempts)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or(Socks5Error::ConnectionFailed("Max retries exceeded".to_string())))
    }

    pub async fn start_reverse_tunnel(&self, local_port: u16) -> Result<(), Socks5Error> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", local_port))
            .await
            .map_err(|e| Socks5Error::ConnectionFailed(format!("Failed to bind local port: {}", e)))?;

        println!("[SOCKS5] Started reverse tunnel on local port {}", local_port);

        loop {
            match listener.accept().await {
                Ok((local_stream, addr)) => {
                    println!("[SOCKS5] Accepted connection from {}", addr);
                    let client = self.clone();
                    
                    // Handle each connection in a separate task
                    tokio::spawn(async move {
                        if let Err(e) = client.handle_reverse_connection(local_stream).await {
                            println!("[ERROR] Reverse tunnel connection failed: {}", e);
                        }
                    });
                }
                Err(e) => {
                    println!("[ERROR] Failed to accept connection: {}", e);
                }
            }
        }
    }

    async fn handle_reverse_connection(&self, mut local_stream: TcpStream) -> Result<(), Socks5Error> {
        // Read SOCKS5 request from local client
        let mut buf = [0u8; 4];
        local_stream.read_exact(&mut buf).await
            .map_err(|e| Socks5Error::ConnectionFailed(format!("Failed to read request: {}", e)))?;

        if buf[0] != SOCKS5_VERSION {
            return Err(Socks5Error::InvalidAddress("Invalid SOCKS5 version".to_string()));
        }

        // Read target address
        let addr_type = match local_stream.read_u8().await {
            Ok(t) => t,
            Err(e) => return Err(Socks5Error::ConnectionFailed(format!("Failed to read address type: {}", e))),
        };

        let target_addr = match addr_type {
            ADDR_TYPE_IPV4 => {
                let mut addr = [0u8; 4];
                local_stream.read_exact(&mut addr).await
                    .map_err(|e| Socks5Error::ConnectionFailed(format!("Failed to read IPv4: {}", e)))?;
                format!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3])
            },
            ADDR_TYPE_DOMAIN => {
                let len = local_stream.read_u8().await
                    .map_err(|e| Socks5Error::ConnectionFailed(format!("Failed to read domain length: {}", e)))? as usize;
                let mut domain = vec![0u8; len];
                local_stream.read_exact(&mut domain).await
                    .map_err(|e| Socks5Error::ConnectionFailed(format!("Failed to read domain: {}", e)))?;
                String::from_utf8(domain)
                    .map_err(|e| Socks5Error::InvalidAddress(format!("Invalid domain name: {}", e)))?
            },
            _ => return Err(Socks5Error::InvalidAddress("Unsupported address type".to_string())),
        };

        let target_port = local_stream.read_u16().await
            .map_err(|e| Socks5Error::ConnectionFailed(format!("Failed to read port: {}", e)))?;

        // Connect to target through SOCKS5 proxy
        let remote_stream = self.connect_to(target_addr.clone(), target_port).await?;

        // Send success response
        let response = [
            SOCKS5_VERSION,
            REP_SUCCESS,
            0x00, // Reserved
            ADDR_TYPE_IPV4,
            0, 0, 0, 0, // Bind address
            0, 0, // Bind port
        ];
        local_stream.write_all(&response).await
            .map_err(|e| Socks5Error::ConnectionFailed(format!("Failed to send response: {}", e)))?;

        // Start bidirectional forwarding
        let (mut local_read, mut local_write) = local_stream.into_split();
        let (mut remote_read, mut remote_write) = remote_stream.into_split();

        let client_to_target = tokio::spawn(async move {
            tokio::io::copy(&mut local_read, &mut remote_write).await
        });

        let target_to_client = tokio::spawn(async move {
            tokio::io::copy(&mut remote_read, &mut local_write).await
        });

        // Wait for either direction to complete
        tokio::select! {
            _ = client_to_target => {},
            _ = target_to_client => {},
        }

        Ok(())
    }
}

pub async fn socks5_bind(
    proxy_addr: &str,
    proxy_port: u16,
    bind_port: u16,
) -> io::Result<TcpStream> {
    // 1) CONNECT to the proxy
    let mut stream = TcpStream::connect((proxy_addr, proxy_port)).await?;

    // 2) NO‑AUTH handshake
    stream.write_all(&[0x05, 0x01, 0x00]).await?;
    let mut resp = [0u8; 2];
    stream.read_exact(&mut resp).await?;
    if resp != [0x05, 0x00] {
        return Err(io::Error::new(io::ErrorKind::Other, format!("handshake failed: {:?}", resp)));
    }

    // 3) REQUEST BIND to 0.0.0.0:bind_port
    let mut req = vec![0x05, 0x02, 0x00, 0x01];
    req.extend(&[0, 0, 0, 0]);                // IPv4 “0.0.0.0”
    req.extend(&bind_port.to_be_bytes());     // port
    stream.write_all(&req).await?;

    // 4) FIRST reply (bound address)
    let mut hdr = [0u8; 4];
    stream.read_exact(&mut hdr).await?;
    if hdr[1] != 0x00 {
        return Err(io::Error::new(io::ErrorKind::Other, format!("bind failed: {:?}", hdr)));
    }
    // skip the bound‐address block
    let skip = match hdr[3] {
        0x01 => 4 + 2,
        0x04 => 16 + 2,
        0x03 => {
            let mut len = [0u8]; stream.read_exact(&mut len).await?;
            (len[0] as usize) + 2
        }
        _ => return Err(io::Error::new(io::ErrorKind::Other, "bad ATYP")),
    };
    let mut junk = vec![0u8; skip];
    stream.read_exact(&mut junk).await?;

    // 5) SECOND reply (incoming connect from server)
    let mut hdr2 = [0u8; 4];
    stream.read_exact(&mut hdr2).await?;
    if hdr2[1] != 0x00 {
        return Err(io::Error::new(io::ErrorKind::Other, format!("bind connect failed: {:?}", hdr2)));
    }
    // skip the source‐address block
    let skip2 = match hdr2[3] {
        0x01 => 4 + 2,
        0x04 => 16 + 2,
        0x03 => {
            let mut len = [0u8]; stream.read_exact(&mut len).await?;
            (len[0] as usize) + 2
        }
        _ => return Err(io::Error::new(io::ErrorKind::Other, "bad ATYP2")),
    };
    let mut junk2 = vec![0u8; skip2];
    stream.read_exact(&mut junk2).await?;

    // 6) Now `stream` is your two‑way channel: server ↔ agent
    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_socks5_connection() {
        let client = Socks5Client::new("127.0.0.1".to_string(), 1080)
            .with_timeout(Duration::from_secs(5));
        
        let result = client.connect_to("example.com".to_string(), 80).await;
        match result {
            Ok(_) => println!("Connection successful"),
            Err(e) => println!("Connection failed: {}", e),
        }
    }

    #[tokio::test]
    async fn test_socks5_auth_connection() {
        let client = Socks5Client::new("127.0.0.1".to_string(), 1080)
            .with_auth("user".to_string(), "pass".to_string())
            .with_timeout(Duration::from_secs(5));
        
        let result = client.connect_with_retries("example.com".to_string(), 80, 3).await;
        match result {
            Ok(_) => println!("Authenticated connection successful"),
            Err(e) => println!("Authenticated connection failed: {}", e),
        }
    }
}

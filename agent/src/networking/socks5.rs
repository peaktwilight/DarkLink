use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;
use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use thiserror::Error;

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

        let stream = match (&self.username, &self.password) {
            (Some(user), Some(pass)) => {
                tokio::time::timeout(
                    self.timeout,
                    Socks5Stream::connect_with_password(
                        addr,
                        target_addr.clone(),
                        target_port,
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
                        target_addr.clone(),
                        target_port,
                    )
                ).await
                .map_err(|_| Socks5Error::Timeout)?
                .map_err(|e| Socks5Error::ConnectionFailed(e.to_string()))?
            }
        };

        let tcp_stream = stream.into_inner();
        
        // Store a clone of the connection in the pool before returning
        if let Ok(cloned_stream) = tcp_stream.try_clone().await {
            self.store_connection(target_key, cloned_stream).await;
        }

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

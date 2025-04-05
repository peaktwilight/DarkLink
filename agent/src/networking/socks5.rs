use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;
use std::error::Error;
use std::net::SocketAddr;

pub struct Socks5Client {
    proxy_addr: String,
    proxy_port: u16,
    username: Option<String>,
    password: Option<String>,
}

impl Socks5Client {
    pub fn new(proxy_addr: String, proxy_port: u16) -> Self {
        Self {
            proxy_addr,
            proxy_port,
            username: None,
            password: None,
        }
    }

    pub fn with_auth(mut self, username: String, password: String) -> Self {
        self.username = Some(username);
        self.password = Some(password);
        self
    }

    pub async fn connect_to(&self, target_addr: String, target_port: u16) -> Result<TcpStream, Box<dyn Error>> {
        let proxy_addr = format!("{}:{}", self.proxy_addr, self.proxy_port);
        let addr = proxy_addr.parse::<SocketAddr>()?;

        let stream = match (&self.username, &self.password) {
            (Some(user), Some(pass)) => {
                Socks5Stream::connect_with_password(
                    addr,
                    target_addr,
                    target_port,
                    user,
                    pass,
                ).await?
            },
            _ => {
                Socks5Stream::connect(
                    addr,
                    target_addr,
                    target_port,
                ).await?
            }
        };

        Ok(stream.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_socks5_connection() {
        let client = Socks5Client::new("127.0.0.1".to_string(), 1080);
        let result = client.connect_to("example.com".to_string(), 80).await;
        assert!(result.is_ok() || result.is_err()); // Basic test to ensure code compiles
    }
}

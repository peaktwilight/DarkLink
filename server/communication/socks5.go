package communication

import (
	"context"
	"fmt"
	"net"
	"time"

	"golang.org/x/net/proxy"
)

// SOCKS5Client provides a SOCKS5 proxy client
type SOCKS5Client struct {
	proxyAddr string
	proxyPort int
	username  string
	password  string
	dialer    proxy.Dialer
	timeout   time.Duration
}

// NewSOCKS5Client creates a new SOCKS5 client
func NewSOCKS5Client(proxyAddr string, proxyPort int, timeout time.Duration) *SOCKS5Client {
	return &SOCKS5Client{
		proxyAddr: proxyAddr,
		proxyPort: proxyPort,
		timeout:   timeout,
	}
}

// WithAuth adds authentication to the SOCKS5 client
func (c *SOCKS5Client) WithAuth(username, password string) *SOCKS5Client {
	c.username = username
	c.password = password
	return c
}

// Connect establishes a connection through the SOCKS5 proxy
func (c *SOCKS5Client) Connect() error {
	auth := &proxy.Auth{}
	if c.username != "" && c.password != "" {
		auth.User = c.username
		auth.Password = c.password
	} else {
		auth = nil
	}

	proxyDialer, err := proxy.SOCKS5("tcp", fmt.Sprintf("%s:%d", c.proxyAddr, c.proxyPort), auth, proxy.Direct)
	if err != nil {
		return fmt.Errorf("failed to create SOCKS5 dialer: %v", err)
	}

	c.dialer = proxyDialer
	return nil
}

// Dial connects to the target address through the SOCKS5 proxy
func (c *SOCKS5Client) Dial(network, addr string) (net.Conn, error) {
	if c.dialer == nil {
		if err := c.Connect(); err != nil {
			return nil, err
		}
	}

	ctx, cancel := context.WithTimeout(context.Background(), c.timeout)
	defer cancel()

	return c.dialer.(proxy.ContextDialer).DialContext(ctx, network, addr)
}

// Close cleans up any resources used by the client
func (c *SOCKS5Client) Close() {
	// Currently no cleanup needed, but implemented for future use
	c.dialer = nil
}

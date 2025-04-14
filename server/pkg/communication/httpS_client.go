package communication

import (
	"crypto/tls"
	"fmt"
	"io"
	"net/http"
	"time"
)

// HTTPSClient provides a secure HTTP client with configurable TLS settings
type HTTPSClient struct {
	client *http.Client
}

// NewHTTPSClient creates a new HTTPS client with custom configuration
func NewHTTPSClient(insecureSkipVerify bool, timeout time.Duration) *HTTPSClient {
	transport := &http.Transport{
		TLSClientConfig: &tls.Config{
			InsecureSkipVerify: insecureSkipVerify,
		},
	}

	client := &http.Client{
		Transport: transport,
		Timeout:   timeout,
	}

	return &HTTPSClient{
		client: client,
	}
}

// Get performs an HTTPS GET request
func (c *HTTPSClient) Get(url string, headers map[string]string) (*http.Response, error) {
	req, err := http.NewRequest(http.MethodGet, url, nil)
	if err != nil {
		return nil, fmt.Errorf("error creating request: %v", err)
	}

	for key, value := range headers {
		req.Header.Set(key, value)
	}

	return c.client.Do(req)
}

// Post performs an HTTPS POST request
func (c *HTTPSClient) Post(url string, body io.Reader, headers map[string]string) (*http.Response, error) {
	req, err := http.NewRequest(http.MethodPost, url, body)
	if err != nil {
		return nil, fmt.Errorf("error creating request: %v", err)
	}

	for key, value := range headers {
		req.Header.Set(key, value)
	}

	return c.client.Do(req)
}

// Put performs an HTTPS PUT request
func (c *HTTPSClient) Put(url string, body io.Reader, headers map[string]string) (*http.Response, error) {
	req, err := http.NewRequest(http.MethodPut, url, body)
	if err != nil {
		return nil, fmt.Errorf("error creating request: %v", err)
	}

	for key, value := range headers {
		req.Header.Set(key, value)
	}

	return c.client.Do(req)
}

// Delete performs an HTTPS DELETE request
func (c *HTTPSClient) Delete(url string, headers map[string]string) (*http.Response, error) {
	req, err := http.NewRequest(http.MethodDelete, url, nil)
	if err != nil {
		return nil, fmt.Errorf("error creating request: %v", err)
	}

	for key, value := range headers {
		req.Header.Set(key, value)
	}

	return c.client.Do(req)
}

// Close cleans up any resources used by the client
func (c *HTTPSClient) Close() {
	c.client.CloseIdleConnections()
}

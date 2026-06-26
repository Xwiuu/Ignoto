package adapters

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"net"
	"net/http"
	"time"

	"golang.org/x/net/proxy"
)

// TorHTTPClient routes HTTP traffic exclusively through a Tor SOCKS5 proxy.
type TorHTTPClient struct {
	client    *http.Client
	targetURL string
}

// NewTorHTTPClient constructs a new TorHTTPClient configured with the given SOCKS5 proxy and target URL.
// It implements a strict Fail-Close policy: if the proxy is down, connection fails and never leaks to clean net.
func NewTorHTTPClient(proxyAddr string, targetURL string) (*TorHTTPClient, error) {
	// Configure the SOCKS5 dialer. Passing proxy.Direct as the forward dialer means
	// the connection to the SOCKS5 server itself is direct, but all traffic routed through the
	// returned dialer will be tunneled. Names are resolved by Tor, avoiding local DNS leaks.
	dialer, err := proxy.SOCKS5("tcp", proxyAddr, nil, proxy.Direct)
	if err != nil {
		return nil, fmt.Errorf("failed to build SOCKS5 dialer: %w", err)
	}

	// We type-assert the dialer to proxy.ContextDialer to respect Context cancellation/timeouts.
	var dialContext func(ctx context.Context, network, address string) (net.Conn, error)
	if contextDialer, ok := dialer.(proxy.ContextDialer); ok {
		dialContext = contextDialer.DialContext
	} else {
		dialContext = func(ctx context.Context, network, address string) (net.Conn, error) {
			return dialer.Dial(network, address)
		}
	}

	// Force HTTP transport to use exclusively the SOCKS5 dialer. No fallback dialer is provided.
	// This achieves the Fail-Close requirement.
	transport := &http.Transport{
		DialContext:         dialContext,
		MaxIdleConns:        100,
		IdleConnTimeout:     90 * time.Second,
		TLSHandshakeTimeout: 10 * time.Second,
	}

	client := &http.Client{
		Transport: transport,
		Timeout:   30 * time.Second,
	}

	return &TorHTTPClient{
		client:    client,
		targetURL: targetURL,
	}, nil
}

// Post sends a POST request with the given JSON payload to the preconfigured target URL.
func (c *TorHTTPClient) Post(ctx context.Context, payload []byte) ([]byte, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, c.targetURL, bytes.NewReader(payload))
	if err != nil {
		return nil, fmt.Errorf("failed to create http request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to route request through Tor proxy: %w", err)
	}
	defer resp.Body.Close()

	respBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read response body: %w", err)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return respBytes, fmt.Errorf("target node returned non-success code %d: %s", resp.StatusCode, string(respBytes))
	}

	return respBytes, nil
}

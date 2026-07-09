package core

import (
	"context"
	"encoding/json"
	"fmt"
)

// Transaction represents a generic JSON-RPC request payload.
type Transaction struct {
	JSONRPC string          `json:"jsonrpc"`
	Method  string          `json:"method"`
	Params  json.RawMessage `json:"params,omitempty"`
	ID      interface{}     `json:"id,omitempty"`
}

// TorClientPort defines the outbound boundary interface (Port) for routing payloads over Tor SOCKS5 proxy.
type TorClientPort interface {
	Post(ctx context.Context, payload []byte) ([]byte, int, error)
}

// RouterUseCase orchestrates the routing of transactions from local ports to external networks.
type RouterUseCase struct {
	torClient TorClientPort
}

// NewRouterUseCase initializes a new RouterUseCase with the specified outbound Tor client adapter.
func NewRouterUseCase(torClient TorClientPort) *RouterUseCase {
	return &RouterUseCase{
		torClient: torClient,
	}
}

// RouteTransaction serializes the transaction and uses the Tor client port to dispatch it to the blockchain node.
func (uc *RouterUseCase) RouteTransaction(ctx context.Context, tx Transaction) ([]byte, int, error) {
	payload, err := json.Marshal(tx)
	if err != nil {
		return nil, 0, fmt.Errorf("failed to marshal transaction payload: %w", err)
	}

	response, statusCode, err := uc.torClient.Post(ctx, payload)
	if err != nil {
		return nil, statusCode, fmt.Errorf("transaction routing use case failed: %w", err)
	}

	return response, statusCode, nil
}

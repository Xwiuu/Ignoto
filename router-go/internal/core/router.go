package core

import (
	"context"
	"encoding/json"
	"fmt"
)

// Transaction represents the data payload containing commitments, nullifiers, and the ZK Proof.
type Transaction struct {
	InputsCommitment  [2]string `json:"inputs_commitment"`
	OutputsCommitment [2]string `json:"outputs_commitment"`
	InputsNullifier   [2]string `json:"inputs_nullifier"`
	Proof             string    `json:"proof"`
}

// TorClientPort defines the outbound boundary interface (Port) for routing payloads over Tor SOCKS5 proxy.
type TorClientPort interface {
	Post(ctx context.Context, payload []byte) ([]byte, error)
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
func (uc *RouterUseCase) RouteTransaction(ctx context.Context, tx Transaction) ([]byte, error) {
	payload, err := json.Marshal(tx)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal transaction payload: %w", err)
	}

	response, err := uc.torClient.Post(ctx, payload)
	if err != nil {
		return nil, fmt.Errorf("transaction routing use case failed: %w", err)
	}

	return response, nil
}

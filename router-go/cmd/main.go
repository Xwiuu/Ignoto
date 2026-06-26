package main

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/Xwiuu/Ignoto/router-go/internal/adapters"
	"github.com/Xwiuu/Ignoto/router-go/internal/core"
	"github.com/Xwiuu/Ignoto/router-go/internal/ports"
)

func main() {
	log.Println("[Ignoto] Initializing Ghost Router (OpSec Onion Relayer)...")

	// 1. Resolve configuration from environment variables with safe defaults
	proxyAddr := os.Getenv("TOR_PROXY_ADDR")
	if proxyAddr == "" {
		proxyAddr = "127.0.0.1:9050" // Default SOCKS5 proxy port for Tor local daemon
	}

	targetNodeURL := os.Getenv("TARGET_NODE_URL")
	if targetNodeURL == "" {
		targetNodeURL = "http://127.0.0.1:9944" // Default Substrate/Ignoto blockchain RPC node
	}

	serverPort := os.Getenv("PORT")
	if serverPort == "" {
		serverPort = "8080"
	}

	log.Printf("[Ignoto] Configured Tor SOCKS5 Proxy: %s", proxyAddr)
	log.Printf("[Ignoto] Configured Blockchain Target Node: %s", targetNodeURL)
	log.Printf("[Ignoto] Configured Server Port: %s", serverPort)

	// 2. Initialize outbound Tor SOCKS5 adapter
	torHTTPClient, err := adapters.NewTorHTTPClient(proxyAddr, targetNodeURL)
	if err != nil {
		log.Fatalf("[Ignoto] Fatal error initializing Tor client: %v", err)
	}

	// 3. Inject adapter into core Use Case (dependency inversion)
	routerUseCase := core.NewRouterUseCase(torHTTPClient)

	// 4. Inject Use Case into primary HTTP handler Port
	httpHandler := ports.NewHTTPHandler(routerUseCase)

	// 5. Setup Router / Multiplexer
	mux := http.NewServeMux()
	
	// Transaction Routing endpoint
	mux.HandleFunc("/route-transaction", httpHandler.RouteTransactionHandler)
	
	// Local health check
	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte(`{"status":"healthy","tor_proxy":"configured"}`))
	})

	serverAddr := fmt.Sprintf(":%s", serverPort)
	server := &http.Server{
		Addr:         serverAddr,
		Handler:      mux,
		ReadTimeout:  15 * time.Second,
		WriteTimeout: 15 * time.Second,
		IdleTimeout:  60 * time.Second,
	}

	// Channel to capture OS signals for graceful shutdown
	stopChan := make(chan os.Signal, 1)
	signal.Notify(stopChan, os.Interrupt, syscall.SIGTERM)

	// Run the HTTP server in a goroutine
	go func() {
		log.Printf("[Ignoto] Ghost Router server successfully listening at http://localhost%s", serverAddr)
		if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("[Ignoto] Fatal server failure: %v", err)
		}
	}()

	// Block until signal is received
	<-stopChan
	log.Println("[Ignoto] Shutting down Ghost Router server gracefully...")

	// Create a context with a timeout for shutdown
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	if err := server.Shutdown(ctx); err != nil {
		log.Fatalf("[Ignoto] Server forced to shutdown: %v", err)
	}

	log.Println("[Ignoto] Ghost Router server stopped.")
}

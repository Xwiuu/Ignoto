# Ignoto Protocol - Tor Hidden Service & Go Router Integration

This document outlines the steps taken to configure the Tor Hidden Service, resolve the Docker container bootstrap failures, update the Go Router, integrate a Node.js/TypeScript client script, and validate end-to-end transaction routing.

---

## 🛠️ Summary of Changes

### 1. Modernized Tor Proxy Configuration
* **Issue**: The original `dperson/torproxy` container failed to bootstrap (stuck at 30% loading consensus directory) because its hardcoded directory authority certificates had expired (as of 2026).
* **Solution**: Developed a custom, lightweight Alpine-based Tor Dockerfile at [Dockerfile](file:///D:/ignoto/infrastructure/tor/Dockerfile) that installs the latest Tor client (`0.4.9.11-r0`) with updated certificates.
* **Volume/Socket Perms**: Configured the control sockets and authorization files to reside in `/var/lib/tor/run/` to avoid permission issues under user `tor`.

### 2. Docker Compose Configuration
* Updated [docker-compose.yml](file:///D:/ignoto/docker-compose.yml) to build the custom Tor proxy service and mount the custom [torrc](file:///D:/ignoto/infrastructure/tor/torrc) configuration:
```yaml
  tor-proxy:
    build:
      context: ./infrastructure/tor
      dockerfile: Dockerfile
    container_name: ignoto-tor
    ports:
      - "9050:9050"
    volumes:
      - ./infrastructure/tor/torrc:/etc/tor/torrc:ro
    networks:
      - ignoto-net
```

### 3. Go Router Refactoring (Blind Relay)
* **Generic JSON-RPC Struct**: Refactored `Transaction` in [router.go](file:///D:/ignoto/router-go/internal/core/router.go) to remove strict fields (e.g. `inputs_commitment`) and map to a generic JSON-RPC structure:
  ```go
  type Transaction struct {
  	JSONRPC string          `json:"jsonrpc"`
  	Method  string          `json:"method"`
  	Params  json.RawMessage `json:"params,omitempty"`
  	ID      interface{}     `json:"id,omitempty"`
  }
  ```
* **Removed Strict Validation**: Deleted `decoder.DisallowUnknownFields()` from [http.go](file:///D:/ignoto/router-go/internal/ports/http.go) to avoid rejecting incoming payloads with unknown/custom parameters.
* **HTTP Status Code & Response Relaying**: Refactored the `TorHTTPClient.Post` method in [tor_client.go](file:///D:/ignoto/router-go/internal/adapters/tor_client.go) to return the raw response body and HTTP status code directly back to the client, without returning errors on non-2xx statuses (allowing standard HTTP error mapping).
* **Timeout Adjustment**: Increased the HTTP server `ReadTimeout` and `WriteTimeout` to `60 * time.Second` to ensure Tor has enough time to establish circuits and publish descriptors without the Go server aborting the connection prematurely.

### 4. Node.js & TypeScript Client Integration
* Created a TypeScript client project at [client-node](file:///D:/ignoto/client-node) installing dependencies `@polkadot/api` and `axios`.
* Developed [submit-tx.ts](file:///D:/ignoto/client-node/submit-tx.ts) which:
  1. Establishes a WebSocket connection to the local Substrate node (`ws://127.0.0.1:9944`) to read metadata.
  2. Creates mock bytes in hex (`inputsCommitment`, `outputsCommitment`, `inputsNullifier`, and a `proof` array).
  3. Builds the signed extrinsic (`api.tx.ignoto.transferShielded`) using the standard Alice keypair.
  4. Generates the pure SCALE-encoded hex string from the extrinsic (`tx.toHex()`).
  5. Formulates the JSON-RPC envelope: `{"jsonrpc": "2.0", "method": "author_submitExtrinsic", "params": ["<HEX>"], "id": 1}`.
  6. Sends the envelope to the Go Router via HTTP POST.

---

## 🔑 Generated Onion Service Address
The unique cryptographic domain generated for the blockchain node is:
```text
ka7vtki3zksdwqgzzffuswedufqkxc7ab42f4wec325rkv7bk2ine4id.onion
```

---

## 🧪 Verification & Testing

### 1. End-to-End Client Transaction Submission
Executed the TypeScript client script (`npx ts-node submit-tx.ts`):
```text
🚀 Initializing Ignoto Client...
🛰️ Connected to Substrate Node via WebSocket.

📖 Pallet Metadata Definition:
 - proofBytes: Bytes
 - vkBytes: Bytes
 - inputsCommitment: [[u8;32];2]
 - outputsCommitment: [[u8;32];2]
 - inputsNullifier: [[u8;32];2]

🛠️ Constructing extrinsic call with 5 arguments...
🔑 Signing transaction using Alice developer account (5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY)...

📦 Generated SCALE-encoded Extrinsic Hex:
0xb5078400d43593... (SCALE encoded bytes)

🔌 Disconnected from Substrate WebSocket.

📤 Dispatching JSON-RPC envelope to Go Router (http://localhost:8080/route-transaction)...
✅ Final response received from Go Router:
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": "0x9aec354abb39f841fc26c28ede5ba0f054d48b4e13d44594c44ec0620668af49"
}
```
* **Result**: The transaction was successfully signed, serialized to SCALE codec, routed through the Go Router, pushed over SOCKS5 and Tor, and emerged at the Rust node which added it to the transaction pool, returning the transaction hash `0x9aec354a...`.

### 2. Rust Node Block Inclusion Verification
Checked the container logs of `ignoto-node` post-submission:
```text
2026-07-09 03:27:45 🎁 Prepared block for proposing at 527 (5 ms) hash: 0x22dbb04e88f9ffaba9a5fb17dc98184a3c3e12d945b757e9157a7485fbc86762; parent_hash: 0xa148…7a11; end: NoMoreTransactions; extrinsics_count: 2
2026-07-09 03:27:45 Consensus with no RPC sender success: CreatedBlock { hash: 0x22dbb04e88f9ffaba9a5fb17dc98184a3c3e12d945b757e9157a7485fbc86762, aux: ... }
2026-07-09 03:27:45 🏆 Imported #527 (0xa148…7a11 → 0x22db…6762)
```
* **Result**: Block 527 successfully included `extrinsics_count: 2` (1 timestamp inherent + our 1 routed transferShielded transaction), validating complete end-to-end block inclusion of private UTXO transfers via the Tor onion network.

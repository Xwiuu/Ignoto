package ports

import (
	"encoding/json"
	"net/http"

	"github.com/Xwiuu/Ignoto/router-go/internal/core"
)

// HTTPHandler is the primary driving adapter for handling HTTP requests.
type HTTPHandler struct {
	useCase *core.RouterUseCase
}

// NewHTTPHandler constructs a new HTTPHandler injected with the RouterUseCase.
func NewHTTPHandler(useCase *core.RouterUseCase) *HTTPHandler {
	return &HTTPHandler{
		useCase: useCase,
	}
}

// JSONError helper sends a formatted JSON error response.
func (h *HTTPHandler) sendJSONError(w http.ResponseWriter, message string, statusCode int) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(statusCode)
	_ = json.NewEncoder(w).Encode(map[string]string{"error": message})
}

// RouteTransactionHandler receives, parses and routes the transaction payload to the core usecase.
func (h *HTTPHandler) RouteTransactionHandler(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		h.sendJSONError(w, "Method not allowed. Use POST.", http.StatusMethodNotAllowed)
		return
	}

	var tx core.Transaction
	decoder := json.NewDecoder(r.Body)
	decoder.DisallowUnknownFields() // enforce strict payload matching

	if err := decoder.Decode(&tx); err != nil {
		h.sendJSONError(w, "Invalid transaction payload: "+err.Error(), http.StatusBadRequest)
		return
	}

	// Delegate processing to the core usecase
	response, err := h.useCase.RouteTransaction(r.Context(), tx)
	if err != nil {
		h.sendJSONError(w, "Failed to route transaction: "+err.Error(), http.StatusInternalServerError)
		return
	}

	// Write successful response back to the client
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	if len(response) > 0 {
		_, _ = w.Write(response)
	} else {
		_, _ = w.Write([]byte(`{"status":"success"}`))
	}
}

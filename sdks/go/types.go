package forgesdk

// ── Response types ──────────────────────────────────────────────────────────

// StatusInfo represents the result of a Status call.
type StatusInfo struct {
	State string `json:"state"`
}

// SyncReport represents the result of a Sync call.
type SyncReport struct {
	Status string `json:"status,omitempty"`
}

// RepairReport represents the result of a Repair call.
type RepairReport struct {
	Status string `json:"status,omitempty"`
}

// CleanReport represents the result of a Clean call.
type CleanReport struct {
	Status string `json:"status,omitempty"`
}

// RuntimeExplanation describes a runtime's configuration.
type RuntimeExplanation struct {
	Runtime     string   `json:"runtime"`
	State       string   `json:"state"`
	Diagnostics []string `json:"diagnostics"`
}

// OperationSummary describes a single history entry.
type OperationSummary struct {
	ID         string `json:"id"`
	Runtime    string `json:"runtime"`
	DurationMs uint64 `json:"duration_ms"`
	Status     string `json:"status"`
}

// RunOutput describes the result of executing a command.
type RunOutput struct {
	Status     string `json:"status"`
	DurationMs uint64 `json:"duration_ms"`
	Warnings   []string `json:"warnings"`
	Changes    []ChangeEntry `json:"changes"`
	Diagnostics []string `json:"diagnostics"`
}

// ChangeEntry describes a single filesystem change.
type ChangeEntry struct {
	Action string `json:"action"`
	Path   string `json:"path"`
}

// HistoryEntry is a convenience alias.
type HistoryEntry = OperationSummary

// ContextFormat enumerates supported context output formats.
type ContextFormat string

const (
	ContextFormatJSON       ContextFormat = "json"
	ContextFormatJSONPretty ContextFormat = "json-pretty"
	ContextFormatMarkdown   ContextFormat = "markdown"
	ContextFormatMCP        ContextFormat = "mcp"
)

// EnvVar represents a key-value environment variable.
type EnvVar struct {
	Key   string `json:"key"`
	Value string `json:"value"`
}

// ResolvedEnvironment represents the resolved unified environment.
type ResolvedEnvironment struct {
	Vars map[string]string `json:"vars"`
}

// ContextData represents the result of a context query.
type ContextData struct {
	Data string `json:"data"`
}

// ForgeError carries an optional JSON-RPC error code.
type ForgeError struct {
	Code    int    `json:"code,omitempty"`
	Message string `json:"message"`
}

func (e *ForgeError) Error() string {
	return e.Message
}

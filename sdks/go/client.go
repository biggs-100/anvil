package forgesdk

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os/exec"
	"sync"
)

// ── JSON-RPC types ──────────────────────────────────────────────────────────

type rpcRequest struct {
	JSONRPC string      `json:"jsonrpc"`
	ID      int         `json:"id"`
	Method  string      `json:"method"`
	Params  interface{} `json:"params"`
}

type rpcResponse struct {
	JSONRPC string          `json:"jsonrpc"`
	ID      int             `json:"id"`
	Result  json.RawMessage `json:"result,omitempty"`
	Error   *rpcError       `json:"error,omitempty"`
}

type rpcError struct {
	Code    int    `json:"code"`
	Message string `json:"message"`
}

// ── Forge client ────────────────────────────────────────────────────────────

// Forge is a Go SDK client that controls a forge jsonrpc subprocess.
// All methods communicate via JSON-RPC 2.0 over stdin/stdout.
// The client is safe for concurrent use.
type Forge struct {
	cmd    *exec.Cmd
	stdin  io.WriteCloser
	stdout *bufio.Scanner
	mu     sync.Mutex
	nextID int
	cancel context.CancelFunc
	done   chan struct{}
}

// NewForge spawns a forge jsonrpc subprocess and returns a connected client.
// The forge binary must be available on $PATH.
func NewForge() (*Forge, error) {
	ctx, cancel := context.WithCancel(context.Background())
	cmd := exec.CommandContext(ctx, "forge", "jsonrpc")

	stdin, err := cmd.StdinPipe()
	if err != nil {
		cancel()
		return nil, fmt.Errorf("failed to create stdin pipe: %w", err)
	}

	stdout, err := cmd.StdoutPipe()
	if err != nil {
		cancel()
		return nil, fmt.Errorf("failed to create stdout pipe: %w", err)
	}

	// Stderr is inherited from parent process for now
	cmd.Stderr = nil

	if err := cmd.Start(); err != nil {
		cancel()
		return nil, fmt.Errorf("failed to start forge jsonrpc: %w", err)
	}

	f := &Forge{
		cmd:    cmd,
		stdin:  stdin,
		stdout: bufio.NewScanner(stdout),
		cancel: cancel,
		done:   make(chan struct{}),
	}

	// Wait for process exit in background
	go func() {
		cmd.Wait()
		close(f.done)
	}()

	return f, nil
}

// Close terminates the forge subprocess.
func (f *Forge) Close() error {
	f.cancel()
	_ = f.stdin.Close()
	return nil
}

// ── Core RPC method ─────────────────────────────────────────────────────────

func (f *Forge) call(ctx context.Context, method string, params interface{}, result interface{}) error {
	f.mu.Lock()
	f.nextID++
	id := f.nextID
	f.mu.Unlock()

	req := rpcRequest{
		JSONRPC: "2.0",
		ID:      id,
		Method:  method,
		Params:  params,
	}

	reqData, err := json.Marshal(req)
	if err != nil {
		return &ForgeError{Message: fmt.Sprintf("failed to marshal request: %v", err)}
	}

	// Write to stdin
	select {
	case <-ctx.Done():
		return ctx.Err()
	default:
	}

	f.mu.Lock()
	_, err = f.stdin.Write(append(reqData, '\n'))
	f.mu.Unlock()
	if err != nil {
		return &ForgeError{Message: fmt.Sprintf("failed to write request: %v", err)}
	}

	// Read response
	respCh := make(chan *rpcResponse, 1)
	errCh := make(chan error, 1)

	go func() {
		if !f.stdout.Scan() {
			if f.stdout.Err() != nil {
				errCh <- &ForgeError{Message: fmt.Sprintf("failed to read response: %v", f.stdout.Err())}
			} else {
				errCh <- &ForgeError{Message: "forge subprocess closed connection"}
			}
			return
		}

		var resp rpcResponse
		if err := json.Unmarshal([]byte(f.stdout.Text()), &resp); err != nil {
			errCh <- &ForgeError{Message: fmt.Sprintf("failed to parse response: %v", err)}
			return
		}
		respCh <- &resp
	}()

	select {
	case <-ctx.Done():
		return ctx.Err()
	case err := <-errCh:
		return err
	case resp := <-respCh:
		if resp.Error != nil {
			return &ForgeError{
				Code:    resp.Error.Code,
				Message: resp.Error.Message,
			}
		}
		if result != nil {
			if err := json.Unmarshal(resp.Result, result); err != nil {
				return &ForgeError{Message: fmt.Sprintf("failed to decode result: %v", err)}
			}
		}
		return nil
	}
}

// ── Engine methods ──────────────────────────────────────────────────────────

// Status returns the current lifecycle state.
func (f *Forge) Status(ctx context.Context) (*StatusInfo, error) {
	var result StatusInfo
	if err := f.call(ctx, "engine.status", struct{}{}, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Sync synchronises runtimes from lockfile.
func (f *Forge) Sync(ctx context.Context) (*SyncReport, error) {
	var result SyncReport
	if err := f.call(ctx, "engine.sync", struct{}{}, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Repair repairs corrupted or missing runtimes.
func (f *Forge) Repair(ctx context.Context) (*RepairReport, error) {
	var result RepairReport
	if err := f.call(ctx, "engine.repair", struct{}{}, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Clean cleans all local cache and state.
func (f *Forge) Clean(ctx context.Context) (*CleanReport, error) {
	var result CleanReport
	if err := f.call(ctx, "engine.clean", struct{}{}, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Explain returns a runtime's configuration and cache status.
func (f *Forge) Explain(ctx context.Context, runtime string) (*RuntimeExplanation, error) {
	var result RuntimeExplanation
	params := map[string]string{"runtime": runtime}
	if err := f.call(ctx, "engine.explain", params, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// History returns past operations history.
func (f *Forge) History(ctx context.Context, limit int) ([]HistoryEntry, error) {
	var result []HistoryEntry
	params := map[string]int{"limit": limit}
	if err := f.call(ctx, "engine.history", params, &result); err != nil {
		return nil, err
	}
	return result, nil
}

// ── Environment methods ─────────────────────────────────────────────────────

// EnvList returns all environment variables.
func (f *Forge) EnvList(ctx context.Context) (map[string]string, error) {
	var result map[string]string
	if err := f.call(ctx, "env.list", struct{}{}, &result); err != nil {
		return nil, err
	}
	return result, nil
}

// EnvGet returns a single environment variable.
func (f *Forge) EnvGet(ctx context.Context, key string) (*string, error) {
	var result *string
	params := map[string]string{"key": key}
	if err := f.call(ctx, "env.get", params, &result); err != nil {
		return nil, err
	}
	return result, nil
}

// EnvSet sets an environment variable.
func (f *Forge) EnvSet(ctx context.Context, key, value string) error {
	params := map[string]string{"key": key, "value": value}
	return f.call(ctx, "env.set", params, nil)
}

// EnvUnset removes an environment variable.
func (f *Forge) EnvUnset(ctx context.Context, key string) error {
	params := map[string]string{"key": key}
	return f.call(ctx, "env.unset", params, nil)
}

// EnvResolve resolves the unified environment.
func (f *Forge) EnvResolve(ctx context.Context, profile *string) (*ResolvedEnvironment, error) {
	var result ResolvedEnvironment
	params := map[string]*string{"profile": profile}
	if err := f.call(ctx, "env.resolve", params, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// ── Secrets methods ─────────────────────────────────────────────────────────

// SecretSet sets a secret.
func (f *Forge) SecretSet(ctx context.Context, key, value string) error {
	params := map[string]string{"key": key, "value": value}
	return f.call(ctx, "secret.set", params, nil)
}

// SecretGet gets a secret by key.
func (f *Forge) SecretGet(ctx context.Context, key string) (*string, error) {
	var result *string
	params := map[string]string{"key": key}
	if err := f.call(ctx, "secret.get", params, &result); err != nil {
		return nil, err
	}
	return result, nil
}

// SecretList returns all secret keys.
func (f *Forge) SecretList(ctx context.Context) ([]string, error) {
	var result []string
	if err := f.call(ctx, "secret.list", struct{}{}, &result); err != nil {
		return nil, err
	}
	return result, nil
}

// SecretRemove removes a secret.
func (f *Forge) SecretRemove(ctx context.Context, key string) error {
	params := map[string]string{"key": key}
	return f.call(ctx, "secret.remove", params, nil)
}

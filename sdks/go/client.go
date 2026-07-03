package anvilsdk

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

// ── Anvil client ────────────────────────────────────────────────────────────

// Anvil is a Go SDK client that controls a anvil jsonrpc subprocess.
// All methods communicate via JSON-RPC 2.0 over stdin/stdout.
// The client is safe for concurrent use.
type Anvil struct {
	cmd    *exec.Cmd
	stdin  io.WriteCloser
	stdout *bufio.Scanner
	mu     sync.Mutex
	nextID int
	cancel context.CancelFunc
	done   chan struct{}
}

// NewAnvil spawns a anvil jsonrpc subprocess and returns a connected client.
// The anvil binary must be available on $PATH.
func NewAnvil() (*Anvil, error) {
	ctx, cancel := context.WithCancel(context.Background())
	cmd := exec.CommandContext(ctx, "anvil", "jsonrpc")

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
		return nil, fmt.Errorf("failed to start anvil jsonrpc: %w", err)
	}

	a := &Anvil{
		cmd:    cmd,
		stdin:  stdin,
		stdout: bufio.NewScanner(stdout),
		cancel: cancel,
		done:   make(chan struct{}),
	}

	// Wait for process exit in background
	go func() {
		cmd.Wait()
		close(a.done)
	}()

	return a, nil
}

// Close terminates the anvil subprocess.
func (a *Anvil) Close() error {
	a.cancel()
	_ = a.stdin.Close()
	return nil
}

// ── Core RPC method ─────────────────────────────────────────────────────────

func (a *Anvil) call(ctx context.Context, method string, params interface{}, result interface{}) error {
	a.mu.Lock()
	a.nextID++
	id := a.nextID
	a.mu.Unlock()

	req := rpcRequest{
		JSONRPC: "2.0",
		ID:      id,
		Method:  method,
		Params:  params,
	}

	reqData, err := json.Marshal(req)
	if err != nil {
		return &AnvilError{Message: fmt.Sprintf("failed to marshal request: %v", err)}
	}

	// Write to stdin
	select {
	case <-ctx.Done():
		return ctx.Err()
	default:
	}

	a.mu.Lock()
	_, err = a.stdin.Write(append(reqData, '\n'))
	a.mu.Unlock()
	if err != nil {
		return &AnvilError{Message: fmt.Sprintf("failed to write request: %v", err)}
	}

	// Read response
	respCh := make(chan *rpcResponse, 1)
	errCh := make(chan error, 1)

	go func() {
		if !a.stdout.Scan() {
			if a.stdout.Err() != nil {
				errCh <- &AnvilError{Message: fmt.Sprintf("failed to read response: %v", a.stdout.Err())}
			} else {
				errCh <- &AnvilError{Message: "anvil subprocess closed connection"}
			}
			return
		}

		var resp rpcResponse
		if err := json.Unmarshal([]byte(a.stdout.Text()), &resp); err != nil {
			errCh <- &AnvilError{Message: fmt.Sprintf("failed to parse response: %v", err)}
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
			return &AnvilError{
				Code:    resp.Error.Code,
				Message: resp.Error.Message,
			}
		}
		if result != nil {
			if err := json.Unmarshal(resp.Result, result); err != nil {
				return &AnvilError{Message: fmt.Sprintf("failed to decode result: %v", err)}
			}
		}
		return nil
	}
}

// ── Engine methods ──────────────────────────────────────────────────────────

// Status returns the current lifecycle state.
func (a *Anvil) Status(ctx context.Context) (*StatusInfo, error) {
	var result StatusInfo
	if err := a.call(ctx, "engine.status", struct{}{}, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Sync synchronises runtimes from lockfile.
func (a *Anvil) Sync(ctx context.Context) (*SyncReport, error) {
	var result SyncReport
	if err := a.call(ctx, "engine.sync", struct{}{}, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Repair repairs corrupted or missing runtimes.
func (a *Anvil) Repair(ctx context.Context) (*RepairReport, error) {
	var result RepairReport
	if err := a.call(ctx, "engine.repair", struct{}{}, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Clean cleans all local cache and state.
func (a *Anvil) Clean(ctx context.Context) (*CleanReport, error) {
	var result CleanReport
	if err := a.call(ctx, "engine.clean", struct{}{}, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// Explain returns a runtime's configuration and cache status.
func (a *Anvil) Explain(ctx context.Context, runtime string) (*RuntimeExplanation, error) {
	var result RuntimeExplanation
	params := map[string]string{"runtime": runtime}
	if err := a.call(ctx, "engine.explain", params, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// History returns past operations history.
func (a *Anvil) History(ctx context.Context, limit int) ([]HistoryEntry, error) {
	var result []HistoryEntry
	params := map[string]int{"limit": limit}
	if err := a.call(ctx, "engine.history", params, &result); err != nil {
		return nil, err
	}
	return result, nil
}

// ── Environment methods ─────────────────────────────────────────────────────

// EnvList returns all environment variables.
func (a *Anvil) EnvList(ctx context.Context) (map[string]string, error) {
	var result map[string]string
	if err := a.call(ctx, "env.list", struct{}{}, &result); err != nil {
		return nil, err
	}
	return result, nil
}

// EnvGet returns a single environment variable.
func (a *Anvil) EnvGet(ctx context.Context, key string) (*string, error) {
	var result *string
	params := map[string]string{"key": key}
	if err := a.call(ctx, "env.get", params, &result); err != nil {
		return nil, err
	}
	return result, nil
}

// EnvSet sets an environment variable.
func (a *Anvil) EnvSet(ctx context.Context, key, value string) error {
	params := map[string]string{"key": key, "value": value}
	return a.call(ctx, "env.set", params, nil)
}

// EnvUnset removes an environment variable.
func (a *Anvil) EnvUnset(ctx context.Context, key string) error {
	params := map[string]string{"key": key}
	return a.call(ctx, "env.unset", params, nil)
}

// EnvResolve resolves the unified environment.
func (a *Anvil) EnvResolve(ctx context.Context, profile *string) (*ResolvedEnvironment, error) {
	var result ResolvedEnvironment
	params := map[string]*string{"profile": profile}
	if err := a.call(ctx, "env.resolve", params, &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// ── Secrets methods ─────────────────────────────────────────────────────────

// SecretSet sets a secret.
func (a *Anvil) SecretSet(ctx context.Context, key, value string) error {
	params := map[string]string{"key": key, "value": value}
	return a.call(ctx, "secret.set", params, nil)
}

// SecretGet gets a secret by key.
func (a *Anvil) SecretGet(ctx context.Context, key string) (*string, error) {
	var result *string
	params := map[string]string{"key": key}
	if err := a.call(ctx, "secret.get", params, &result); err != nil {
		return nil, err
	}
	return result, nil
}

// SecretList returns all secret keys.
func (a *Anvil) SecretList(ctx context.Context) ([]string, error) {
	var result []string
	if err := a.call(ctx, "secret.list", struct{}{}, &result); err != nil {
		return nil, err
	}
	return result, nil
}

// SecretRemove removes a secret.
func (a *Anvil) SecretRemove(ctx context.Context, key string) error {
	params := map[string]string{"key": key}
	return a.call(ctx, "secret.remove", params, nil)
}

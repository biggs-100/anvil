# SDK Go Specification

## Purpose

Defines the Go SDK package (`forge-sdk-go`) that enables Go programs to control Forge by spawning a `forge --jsonrpc` subprocess and communicating via JSON-RPC 2.0 over stdio.

## Requirements

### Requirement: Package Structure

The SDK MUST be a Go module at `github.com/user/forge/sdk-go`. It MUST use only Go standard library — no external dependencies.

### Requirement: Subprocess Lifecycle

The SDK MUST spawn `forge --jsonrpc` as a child process using `os/exec`. It MUST kill the subprocess on client `Close()`. It MUST provide a `NewForge() (*Forge, error)` constructor that spawns the subprocess and waits for readiness.

#### Scenario: Connect to forge subprocess

- GIVEN `forge` is installed and available on `$PATH`
- WHEN `NewForge()` is called
- THEN it MUST spawn `forge --jsonrpc` as a child process
- AND return a non-nil `*Forge` with no error

#### Scenario: Handle subprocess crash

- GIVEN a `Forge` client connected to a forge subprocess
- WHEN the subprocess exits unexpectedly (e.g., killed externally)
- AND the client sends a request
- THEN the client MUST return an error wrapping the broken-pipe/EOF condition

### Requirement: Method Surface

The `Forge` struct MUST provide Go-idiomatic methods mirroring the Rust SDK surface, all communicating via JSON-RPC:

- `Status() (*StatusInfo, error)`
- `Sync() (*SyncReport, error)`
- `Repair() (*RepairReport, error)`
- `Clean() (*CleanReport, error)`
- `Run(cmd string, args ...string) (*RunOutput, error)`
- `Context(fmt ContextFormat) (interface{}, error)`
- `Explain(runtime string) (string, error)`
- `History(limit int) ([]HistoryEntry, error)`
- Environment: `EnvList(), EnvGet(key), EnvSet(key, val), EnvUnset(key), EnvResolve(key)`
- Secrets: `SecretSet(key, val), SecretGet(key), SecretList(), SecretRemove(key)`

#### Scenario: Call status via RPC

- GIVEN a connected `Forge` client
- WHEN `Status()` is called
- THEN it MUST send a JSON-RPC request with method `"status"`
- AND return the deserialized `StatusInfo` result

#### Scenario: Call sync via RPC

- GIVEN a connected `Forge` client
- WHEN `Sync()` is called
- THEN it MUST return a `SyncReport` with success/failure details

### Requirement: Context Cancellation

All RPC-calling methods SHOULD accept a `context.Context` parameter and SHOULD cancel the in-flight request when the context is cancelled.

#### Scenario: Context cancellation aborts request

- GIVEN a connected `Forge` client
- WHEN a method is called with a cancelled `context.Context`
- THEN the method MUST return an error wrapping `context.Canceled`

### Requirement: Concurrent Safety

The `Forge` struct MUST be safe for concurrent use. Multiple goroutines MAY call methods simultaneously.

#### Scenario: Concurrent calls do not deadlock

- GIVEN a connected `Forge` client
- WHEN 10 goroutines call `Status()` concurrently
- THEN all 10 MUST return valid results without deadlock or data races

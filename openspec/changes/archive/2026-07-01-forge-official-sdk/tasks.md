# Tasks: Forge Official SDK

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~1,400–1,600 |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (Foundation) → PR 2 (Rust SDK) → PR 3 (Go) → PR 4 (Python) → PR 5 (TypeScript) → PR 6 (Tests) |
| Delivery strategy | single-pr (size:exception pre-authorized) |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: size-exception
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | JSON-RPC server in forge-cli: subcommand, read-dispatch-write loop, method catalog, error codes | PR 1 | Base branch: main. Foundation all SDKs depend on |
| 2 | Rust SDK crate: Forge struct, typed methods, async feature | PR 2 | Depends on forge-core only — independent from PR 1 |
| 3 | Go SDK: go.mod, client.go, types.go, subprocess + JSON-RPC | PR 3 | Depends on PR 1 (needs jsonrpc mode in forge-cli) |
| 4 | Python SDK: pyproject.toml, client.py, types.py | PR 4 | Depends on PR 1 |
| 5 | TypeScript SDK: package.json, client.ts, types.ts | PR 5 | Depends on PR 1 |
| 6 | Integration tests + cross-SDK parity | PR 6 | Depends on all prior PRs |

## Phase 1: Foundation — JSON-RPC Server

- [x] 1.1 Add `JsonRpc` variant to `Commands` enum in `crates/forge-cli/src/main.rs`
- [x] 1.2 Create `crates/forge-cli/src/jsonrpc.rs` — stdin read-dispatch-write loop with tokio
- [x] 1.3 Implement method catalog dispatch: engine.*, exec.*, env.*, secret.*, context.*
- [x] 1.4 Map JSON-RPC 2.0 error codes (-32700 Parse, -32600 Invalid, -32601 NotFound, -32603 Internal, -32000+ custom)
- [x] 1.5 Add shared `Arc<Mutex<BufWriter<Stdout>>>` for ordered response writing
- [x] 1.6 Handle EOF shutdown (exit 0 on stdin close)

## Phase 2: SDK Rust — forge-sdk Crate

- [x] 2.1 Create `crates/forge-sdk/Cargo.toml`, add to `[workspace].members` in root `Cargo.toml`
- [x] 2.2 Create `crates/forge-sdk/src/types.rs` — `ForgeError` (Display + Error), type aliases
- [x] 2.3 Create `crates/forge-sdk/src/lib.rs` — `Forge` struct wrapping `Engine` directly
- [x] 2.4 Implement all typed async methods: status, sync, repair, clean, run, shell, context, explain, history, env.*, secret.*
- [x] 2.5 Add feature-gated async support (`default` sync, `async` feature enables async fns)

## Phase 3: SDK Go

- [x] 3.1 Create `sdks/go/go.mod` — module `github.com/user/forge/sdk-go`, stdlib only
- [x] 3.2 Create `sdks/go/types.go` — Go response structs for all method return types
- [x] 3.3 Create `sdks/go/client.go` — `Forge` struct: subprocess spawn, JSON-RPC over stdio, typed methods, `Close()`
- [x] 3.4 Add `context.Context` parameter to all RPC methods for cancellation
- [x] 3.5 Ensure concurrent-safety (mutex on stdio writes)

## Phase 4: SDK Python

- [x] 4.1 Create `sdks/python/pyproject.toml` — package `forge-sdk`, stdlib only
- [x] 4.2 Create `sdks/python/forge_sdk/types.py` — dataclasses for all response types
- [x] 4.3 Create `sdks/python/forge_sdk/client.py` — `Forge` class: subprocess, JSON-RPC, typed methods, `close()`
- [x] 4.4 Create `sdks/python/forge_sdk/__init__.py` — re-export `Forge`, `ForgeError`, types
- [x] 4.5 Add context manager (`with` block) and async variants (`async_status()`, etc.)

## Phase 5: SDK TypeScript

- [x] 5.1 Create `sdks/typescript/package.json` — npm package `@forge/sdk`, Node 18+
- [x] 5.2 Create `sdks/typescript/tsconfig.json` — target ES2020, ship `.d.ts`
- [x] 5.3 Create `sdks/typescript/src/types.ts` — TS interfaces for all response shapes
- [x] 5.4 Create `sdks/typescript/src/client.ts` — `Forge` class: spawn, JSON-RPC, typed async methods, `disconnect()`
- [x] 5.5 Create `sdks/typescript/src/index.ts` — re-export `Forge`, `ForgeError`, all types

## Phase 6: Testing

- [x] 6.1 Integration test: spawn `forge jsonrpc`, send JSON-RPC requests, verify responses
- [x] 6.2 forge-sdk crate tests — `cargo test -p forge-sdk` covering all typed methods
- [x] 6.3 Go SDK integration test — `go test ./...` with subprocess lifecycle
- [x] 6.4 Python SDK integration test — `pytest` or `unittest` with subprocess lifecycle
- [x] 6.5 TypeScript SDK integration test — Node.js script spawning `forge jsonrpc`, verifying JSON-RPC responses (uses `child_process` directly, no test runner dependency)
- [~] 6.6 Cross-SDK parity test — deferred (requires CI matrix for all 4 SDK binaries)
- [x] 6.7 Subprocess lifecycle error test — forge-kill-mid-request test in `crates/forge-cli/tests/jsonrpc_test.rs`

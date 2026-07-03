# Proposal: Anvil Official SDK

## Intent

Forge Core 1.0 is frozen but only callable from Rust. Teams using Go, Python, or TypeScript cannot integrate Anvil into their tooling. This change makes Anvil programmable from any language via official SDKs.

## Scope

### In Scope
- `anvil-sdk` crate — typed Rust wrapper over `Engine`
- `anvil sdk-server` — JSON-RPC mode over stdio in anvil-cli
- Go client — thin JSON-RPC over stdio
- Python client — same transport model
- TypeScript client — same transport model

### Out of Scope
- TCP/HTTP transport
- GUI or plugin SDK
- Engine reimplementation outside Rust

## Capabilities

### New Capabilities
- `sdk-rust`: Rust SDK crate wrapping Engine operations
- `sdk-transport`: JSON-RPC 2.0 server over stdin/stdout in anvil-cli
- `sdk-go`: Go client — spawn forge, JSON-RPC over stdio
- `sdk-python`: Python client — same transport
- `sdk-typescript`: TypeScript/Node client — same transport

### Modified Capabilities
None — core frozen surface untouched.

## Approach

1. Add `anvil-sdk` crate wrapping `Engine` with a clean public API.
2. Add `sdk-server` to anvil-cli — JSON-RPC dispatcher over stdio.
3. Design schema (methods, types, errors) for forward compatibility.
4. Thin clients in Go, Python, TS — each spawns anvil and sends JSON-RPC.
5. CI for anvil-sdk; optional stages for non-Rust SDKs.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/anvil-sdk/` | New | Rust SDK crate |
| `crates/anvil-cli/src/main.rs` | Modified | Add `sdk-server` subcommand |
| `sdks/go/` | New | Go client SDK |
| `sdks/python/` | New | Python client SDK |
| `sdks/typescript/` | New | TS client SDK |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| JSON-RPC schema brittle to Engine changes | Med | Versioned methods, optional params |
| CI complexity from 3 extra languages | Med | Containerize, make non-Rust CI optional |
| stdio race on concurrent requests | Low | Ordered responses, single dispatch |

## Rollback Plan

- Remove `crates/anvil-sdk/` from workspace
- Revert anvil-cli `sdk-server` subcommand
- Delete `sdks/go/`, `sdks/python/`, `sdks/typescript/`
- No anvil-core surface changes to revert

## Dependencies

- `serde_json` / `serde` for JSON-RPC (already in tree)
- Go/Python/TS: stdlib only — no external deps

## Success Criteria

- [ ] `anvil-sdk` compiles and exposes all Engine operations as a typed API
- [ ] `anvil sdk-server` dispatches all methods and returns typed responses
- [ ] Go/TypeScript SDKs can init Anvil and run sync/status/env/secret
- [ ] Python SDK passes same integration tests
- [ ] All existing CLI commands unchanged

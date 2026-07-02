# Proposal: Forge Official SDK

## Intent

Forge Core 1.0 is frozen but only callable from Rust. Teams using Go, Python, or TypeScript cannot integrate Forge into their tooling. This change makes Forge programmable from any language via official SDKs.

## Scope

### In Scope
- `forge-sdk` crate — typed Rust wrapper over `Engine`
- `forge sdk-server` — JSON-RPC mode over stdio in forge-cli
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
- `sdk-transport`: JSON-RPC 2.0 server over stdin/stdout in forge-cli
- `sdk-go`: Go client — spawn forge, JSON-RPC over stdio
- `sdk-python`: Python client — same transport
- `sdk-typescript`: TypeScript/Node client — same transport

### Modified Capabilities
None — core frozen surface untouched.

## Approach

1. Add `forge-sdk` crate wrapping `Engine` with a clean public API.
2. Add `sdk-server` to forge-cli — JSON-RPC dispatcher over stdio.
3. Design schema (methods, types, errors) for forward compatibility.
4. Thin clients in Go, Python, TS — each spawns forge and sends JSON-RPC.
5. CI for forge-sdk; optional stages for non-Rust SDKs.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/forge-sdk/` | New | Rust SDK crate |
| `crates/forge-cli/src/main.rs` | Modified | Add `sdk-server` subcommand |
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

- Remove `crates/forge-sdk/` from workspace
- Revert forge-cli `sdk-server` subcommand
- Delete `sdks/go/`, `sdks/python/`, `sdks/typescript/`
- No forge-core surface changes to revert

## Dependencies

- `serde_json` / `serde` for JSON-RPC (already in tree)
- Go/Python/TS: stdlib only — no external deps

## Success Criteria

- [ ] `forge-sdk` compiles and exposes all Engine operations as a typed API
- [ ] `forge sdk-server` dispatches all methods and returns typed responses
- [ ] Go/TypeScript SDKs can init Forge and run sync/status/env/secret
- [ ] Python SDK passes same integration tests
- [ ] All existing CLI commands unchanged

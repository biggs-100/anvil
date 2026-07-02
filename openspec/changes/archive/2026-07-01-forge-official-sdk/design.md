# Design: Forge Official SDK

## Technical Approach

Add a JSON-RPC 2.0 server mode to forge-cli (dedicated subcommand), a `forge-sdk` Rust crate wrapping `Engine` directly, and thin SDKs in Go/Python/TypeScript communicating via stdio JSON-RPC. Zero changes to `forge-core` — everything is additive.

## Architecture Decisions

### Decision: JSON-RPC Server Entrypoint

| Option | Tradeoff | Decision |
|--------|----------|----------|
| Global `--jsonrpc` flag | Conflicts with clap subcommand parser, all commands must be gated | Rejected |
| Dedicated `JsonRpc` subcommand | Clean separation, matches existing CLI pattern, no parser conflict | **Chosen** |
| Separate binary (`forge-server`) | Extra build target, duplicates Engine init | Rejected |

### Decision: Event Loop Model

| Option | Tradeoff | Decision |
|--------|----------|----------|
| Sequential per-line dispatch | Simple but blocks the entire server on slow ops | Rejected |
| tokio spawn per request + shared stdout writer | Non-blocking, leverages existing tokio runtime, ordered output | **Chosen** |

Each incoming line spawns a tokio task. All responses go through a shared buffered writer guarded by a mutex — no interleaved output lines. Responses carry the request ID for client-side correlation.

### Decision: RPC Method Namespace

| Prefix | Scope | Examples |
|--------|-------|----------|
| `engine.*` | Core engine lifecycle | `engine.status`, `.sync`, `.repair`, `.clean`, `.explain`, `.history` |
| `exec.*` | Command execution | `exec.run`, `exec.shell` |
| `env.*` | Environment variables | `env.list`, `.get`, `.set`, `.unset`, `.resolve` |
| `secret.*` | Secrets management | `secret.set`, `.get`, `.list`, `.remove` |
| `context.*` | Context queries | `context.get` |

Namespaced methods leave room for future capabilities without breaking existing clients.

### Decision: forge-sdk — Direct Wrapper (Not RPC)

| Option | Tradeoff | Decision |
|--------|----------|----------|
| Wraps Engine via RPC loopback | Unnecessary overhead in same process | Rejected |
| Direct function calls to forge-core | Zero overhead, typed, async-compatible | **Chosen** |
| Sync-only API | Simple but limits Rust consumers | Rejected |
| Feature-gated async (`default`, `async`) | Both paths available, zero-cost when unused | **Chosen** |

### Decision: Non-Rust SDK Transport

Go/Python/TS all use the same pattern: spawn `forge --jsonrpc` via subprocess, write JSON-RPC to stdin, read responses from stdout, kill on close. No external dependencies for any SDK — stdlib only.

## Data Flow

```
┌──────────┐    ┌──────────────┐    ┌──────────────┐
│  Go SDK  │    │ Python SDK   │    │  TS SDK      │
│ (exec)   │    │ (subprocess) │    │  (spawn)     │
└──┬───────┘    └──┬───────────┘    └──┬───────────┘
   │               │                   │
   │   JSON-RPC 2.0 over stdin/stdout  │
   └───────────────┼───────────────────┘
                   ▼
        ┌─────────────────────┐
        │  forge --jsonrpc    │
        │  stdin → dispatch   │
        │  → stdout           │
        └────────┬────────────┘
                 │
        ┌────────▼────────────┐
        │  Engine (async)     │
        └─────────────────────┘

┌──────────────────┐
│ forge-sdk crate  │──→ engine methods (direct calls)
│ (Rust SDK)       │
└──────────────────┘
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `Cargo.toml` | Modify | Add `"crates/forge-sdk"` to workspace members |
| `crates/forge-cli/src/main.rs` | Modify | Add `JsonRpc` subcommand with read-dispatch-write loop |
| `crates/forge-sdk/Cargo.toml` | Create | SDK crate manifest, dep on forge-core |
| `crates/forge-sdk/src/lib.rs` | Create | `Forge` struct wrapping `Engine`, typed methods |
| `crates/forge-sdk/src/types.rs` | Create | `ForgeError`, SDK-specific type aliases |
| `sdks/go/go.mod` | Create | Go module `github.com/user/forge/sdk-go` |
| `sdks/go/client.go` | Create | Go `Forge` struct: spawn, JSON-RPC, typed methods |
| `sdks/go/types.go` | Create | Go response types |
| `sdks/python/pyproject.toml` | Create | Package config for `forge-sdk` |
| `sdks/python/forge_sdk/__init__.py` | Create | Package re-exports |
| `sdks/python/forge_sdk/client.py` | Create | Python `Forge` class with subprocess + JSON-RPC |
| `sdks/python/forge_sdk/types.py` | Create | Python dataclasses |
| `sdks/typescript/package.json` | Create | npm package `@forge/sdk` |
| `sdks/typescript/tsconfig.json` | Create | TS config, target ES2020 |
| `sdks/typescript/src/index.ts` | Create | Package entry, type re-exports |
| `sdks/typescript/src/client.ts` | Create | TS `Forge` class: spawn, JSON-RPC, typed async |
| `sdks/typescript/src/types.ts` | Create | TS interfaces for all responses |

## Interfaces / Contracts

### JSON-RPC Method Catalog

```
engine.status     → {state: String}
engine.sync       → {} (void/ok)
engine.repair     → {} (void/ok)
engine.clean      → {} (void/ok)
engine.explain    → {runtime: String} → RuntimeExplanation
engine.history    → {limit?: usize}   → OperationSummary[]
exec.run          → {cmd: String, args: String[]} → RunOutput
exec.shell        → {} → InteractiveSession
env.list          → {} → HashMap<String, String>
env.get           → {key: String} → Option<String>
env.set           → {key, value} → {}
env.unset         → {key} → {}
env.resolve       → {profile?: String} → ResolvedEnvironment
secret.set        → {key, value} → {}
secret.get        → {key} → Option<String>
secret.list       → {} → String[]
secret.remove     → {key} → {}
context.get       → {format?, scope?, exclude?} → ContextData
```

All error responses follow JSON-RPC 2.0: `{"code": -32603, "message": "..."}`.

### forge-sdk Rust Surface (Core)

```rust
pub struct Forge { engine: Engine }
impl Forge {
    pub fn new() -> Result<Self, ForgeError>;
    pub async fn status(&self) -> Result<String, ForgeError>;
    pub async fn sync(&self) -> Result<(), ForgeError>;
    pub async fn repair(&self) -> Result<(), ForgeError>;
    pub async fn clean(&self) -> Result<(), ForgeError>;
    pub async fn explain(&self, runtime: &str) -> Result<RuntimeExplanation, ForgeError>;
    pub async fn history(&self, limit: Option<usize>) -> Result<Vec<OperationSummary>, ForgeError>;
    pub async fn env_list(&self) -> Result<HashMap<String, String>, ForgeError>;
    pub async fn env_get(&self, key: &str) -> Result<Option<String>, ForgeError>;
    pub async fn env_set(&self, key: &str, val: &str) -> Result<(), ForgeError>;
    pub async fn secret_set(&self, key: &str, val: &str) -> Result<(), ForgeError>;
    pub async fn secret_get(&self, key: &str) -> Result<Option<String>, ForgeError>;
    pub async fn secret_list(&self) -> Result<Vec<String>, ForgeError>;
    pub async fn secret_remove(&self, key: &str) -> Result<(), ForgeError>;
}
```

## Testing Strategy

| Layer | What | Approach |
|-------|------|----------|
| Unit | JSON-RPC request/response parsing | Test serde roundtrips, error code mapping |
| Integration | forge --jsonrpc against real binary | Spawn `cargo run -- jsonrpc`, send requests, verify responses |
| Integration | forge-sdk crate | `cargo test -p forge-sdk` — all Engine methods via typed API |
| E2E | Cross-SDK parity | Run same method catalog against all 4 SDKs, assert identical output shapes |
| E2E | Subprocess lifecycle | Kill forge mid-request, assert SDK returns connection error |

## Migration / Rollout

No migration required. All changes are additive. forge-core surface frozen. Existing CLI commands unchanged. SDKs are entirely new artifacts.

## Open Questions

- [ ] Context method — should `context.get` accept the same format/scope/exclude params as the CLI `context` subcommand, or start simpler?
- [ ] Events streaming (`engine.events`) returns a `Receiver<Event>` — exclude from initial catalog or add a polling variant?
- [ ] `exec.run` / `exec.shell` — implement via forge-core `Operation` traits (matching CLI) or inline within the RPC dispatcher?

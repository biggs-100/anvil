# Design: Forge Context Platform (FCP)

## Technical Approach

FCP implements a trait-based query framework inside `forge-core` to aggregate project state, sanitizing secrets and filtering filesystem layouts, before exporting to CLI/JSON-RPC (MCP) or adapting to custom agent prompt wrappers.

## Architecture Decisions

| Option | Tradeoff | Decision |
|---|---|---|
| **Handshake Capability Negotiation** | Adds RPC roundtrip, but guarantees agent compatibility. | Implement version check and scope matching via JSON-RPC. |
| **In-memory Masking** | Minimal string scanning overhead. | Secrets & Environment providers run regex matching based on `is_secret` dynamically. |
| **Workspace Tree Traversal** | Deep sweeps inflate token consumption. | Limit crawl to depth 5, max 1000 files, and prune binary/cache dirs natively. |

## Data Flow

```
    [Agent / CLI] ──(Handshake / Query)──→ [ContextEngine]
                                                 │
      ┌──────────────┬──────────────┬────────────┼─────────────┬─────────────┐
      ▼              ▼              ▼            ▼             ▼             ▼
  [Runtime]   [Configuration] [Diagnostics] [Workspace]  [Environment]   [Secrets]
      │              │              │            │             │             │
      └──────────────┴──────────────┼────────────┴─────────────┴─────────────┘
                                    ▼ (Filter & Mask Sensitive Data)
                              [ForgeContext]
                                    │
                                    ▼ (Exporter / Adapter)
                             [Target Format]
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/forge-core/src/context/mod.rs` | Create | FCP Core types, traits, engine, and 6 providers. |
| `crates/forge-core/src/lib.rs` | Modify | Re-export `context` module. |
| `crates/forge-cli/src/main.rs` | Modify | Hook up `forge context` subcommand and options parsing. |

## Interfaces / Contracts

```rust
// crates/forge-core/src/context/mod.rs
pub trait ContextProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn collect(&self, options: &ContextOptions) -> Result<serde_json::Value, String>;
}

pub trait ContextExporter: Send + Sync {
    fn name(&self) -> &'static str;
    fn export(&self, context: &ForgeContext) -> Result<String, String>;
}

pub trait AgentAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    fn adapt(&self, context: &ForgeContext, exporter: &dyn ContextExporter) -> Result<String, String>;
}
```

### JSON-RPC Handshake Payload
* **Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "fcp.handshake",
  "params": {
    "version": "1.0.0",
    "capabilities": {
      "scopes": ["workspace", "secrets"],
      "exporters": ["json", "markdown"]
    }
  },
  "id": 1
}
```
* **Response**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "version": "1.0.0",
    "negotiated_capabilities": {
      "scopes": ["workspace", "secrets"],
      "exporters": ["json", "markdown"]
    }
  },
  "id": 1
}
```

## Implementations Detail

### Concrete Providers (6)
1. **Runtime**: Summarizes active runtimes/shims from `forge.lock`.
2. **Configuration**: Parses `forge.toml` active profile configuration.
3. **Diagnostics**: Calls `DiagnosticEngine::run` to fetch health metrics.
4. **Workspace**: Directory crawler using a custom stack/recursion with `max_depth = 5`, `max_files = 1000`, ignoring `.git`, `node_modules`, `target`, and extensions matching binary signatures.
5. **Environment**: Retrieves environment variables. Keys/values matching `is_secret(key)` are replaced with `[MASKED]`.
6. **Secrets**: Lists registered secret keys and their sources (e.g. `keyring`), metadata presence only. Values are strictly masked.

### Exporters & Adapters
* **JsonExporter**: Standard `serde_json` serialization.
* **MarkdownExporter**: Formatted tables and lists for LLM consumption.
* **McpExporter**: Serves context as MCP resources.
* **ClaudeCodeAdapter**: Wraps exporter output in `<forge_context>` XML.
* **GeminiCliAdapter**: Translates to raw JSON context blocks.
* **AiderAdapter**: Serializes Aider-compatible repository structure map.

## CLI Integration
Subcommand `forge context`:
* `--format <json|markdown>` (Default: `json`)
* `--scope <scope>` (Multi-value: `runtime`, `configuration`, `diagnostics`, `workspace`, `environment`, `secrets`, `all`)
* `--exclude <path>` (Filters directories out of workspace crawl)

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| **Unit** | FCP Handshake | Validate JSON-RPC serialization & capability intersection. |
| **Unit** | Provider Concurrency | Query `ContextEngine` with multiple threads verifying thread-safety. |
| **Unit** | Secret Masking | Test `Environment` & `Secrets` providers replace secret patterns with `[MASKED]`. |
| **Unit** | Workspace Limit Bounds | Mock a nested directory and verify scanner cuts off at depth 5 / 1000 files. |
| **Integration** | CLI Outputs | Test `forge context --format json` and `--format markdown` write valid syntax. |

## Migration / Rollout
No migration required.

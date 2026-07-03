# Anvil Core 1.0 — Architecture Freeze

**Status:** Adopted
**Date:** 2026-07-01
**Covenant:** No breaking changes to the frozen surface without a major version bump.
**Scope:** `anvil-core` crate and its public API surface.

---

## Purpose

This document formalizes the Core 1.0 freeze. From this point forward, the core engine is **stable** — its public API, data formats, and behavioral contracts will not break. All future innovation happens in the ecosystem layer (plugins, SDKs, integrations), not in the core.

The freeze is not about stopping development. It is about **creating a trusted foundation** that others can build on without fear of the ground shifting beneath them.

---

## What is Frozen

### 1. Public API — `Engine` Facade

The `Engine` struct in `crates/anvil-core/src/api/v1.rs` is the **sole public entry point**. The following methods are frozen:

| Method | Signature | Purpose |
|--------|-----------|---------|
| `Engine::new` | `(root: PathBuf) -> Result<Self>` | Instantiate engine for a workspace |
| `Engine::get_status` | `() -> Result<String>` | Current lifecycle state |
| `Engine::sync` | `() -> Result<()>` | Sync runtimes from lockfile |
| `Engine::repair` | `() -> Result<()>` | Repair corrupted runtimes |
| `Engine::clean` | `() -> Result<()>` | Clean local cache |
| `Engine::explain` | `(runtime: &str) -> Result<RuntimeExplanation>` | Deep-dive into a runtime |
| `Engine::history` | `(limit: Option<usize>) -> Result<Vec<OperationSummary>>` | Operation history |
| `Engine::trace` | `(id: &str) -> Result<TraceTree>` | Operation tree trace |
| `Engine::events` | `(live: bool) -> Result<Receiver<Event>>` | Event stream |
| `Engine::env_list` | `() -> Result<HashMap<String, String>>` | List env vars |
| `Engine::env_get` | `(key: &str) -> Result<Option<String>>` | Get env var |
| `Engine::env_set` | `(key: &str, value: &str) -> Result<()>` | Set env var |
| `Engine::env_unset` | `(key: &str) -> Result<()>` | Unset env var |
| `Engine::env_resolve` | `(profile: Option<&str>) -> Result<ResolvedEnvironment>` | Resolve full environment |
| `Engine::secret_set` | `(key: &str, value: &str) -> Result<()>` | Set a secret |
| `Engine::secret_get` | `(key: &str) -> Result<Option<String>>` | Get a secret |
| `Engine::secret_list` | `() -> Result<Vec<String>>` | List secrets |
| `Engine::secret_remove` | `(key: &str) -> Result<()>` | Remove a secret |
| `Engine::secret_export` | `() -> Result<HashMap<String, String>>` | Export all secrets |
| `Engine::secret_import` | `(secrets: &HashMap<String, String>) -> Result<()>` | Import secrets |
| `Engine::secret_doctor` | `() -> Result<Vec<String>>` | Keyring health check |

### 2. Core Types

All types exported from `crates/anvil-core/src/types.rs`:

| Type | Role |
|------|------|
| `RuntimeId` | Newtype for runtime names (e.g. `node`, `python`) |
| `RuntimeVersion` | Newtype for version strings |
| `Hash` | Newtype for SHA-256 hex strings |
| `Platform` | Enum: `Windows`, `Macos`, `Linux`, `Unknown` |
| `Architecture` | Enum: `X86_64`, `Aarch64`, `Unknown` |
| `EmulationLog` | Records when a fallback architecture was used |
| `RuntimeLock` | Lockfile entry: name, version, platform, arch, url, size, sha256, emulation |
| `Lockfile` | Container for `Vec<RuntimeLock>` |
| `LifecycleState` | Enum: `Uninitialized` → `Initialized` → ... → `Ready` — full state machine |
| `OperationStatus` | Enum: `Success`, `Failure`, `Warning`, `Skipped` |
| `ChangeRecord` | Path + action ("added", "modified", "deleted") |
| `OperationResult` | Status, duration, warnings, changes, diagnostics |
| `EventStatus` | Enum: `Started`, `Progress(u8)`, `Finished`, `Failed(String)` |
| `Event` | Timestamp, operation_id, runtime, phase, status, message |

### 3. Manifest Format — `anvil.toml`

Stable sections and fields in `crates/anvil-core/src/manifest.rs`:

```toml
[runtimes]        # Map<name, version_req> — required
workspace_id      # Optional string
[config]          # Optional config definitions
  [config.definitions]  # Map<name, ConfigDefinition>
    type          # Optional ("string", "number", "boolean")
    required      # default: false
    default       # Optional TOML value
    pattern       # Optional regex
    description   # Optional string
    secret        # default: false
[profile]         # Optional named profiles
  [profile.<name>]
    [profile.<name>.env]  # Map<string, value>
```

### 4. Lockfile Format — `anvil.lock`

```toml
[[runtime]]
name = "node"
version = "20.11.0"
platform = "windows"
arch = "x86_64"
url = "https://..."
size = 12345678
sha256 = "abc123..."
[emulation]                # Optional — only when architecture differs
requested = "windows-arm64"
installed = "windows-x86_64"
reason = "Native build unavailable"
```

### 5. ACP Handshake Protocol

The handshake schema in `crates/anvil-core/src/context/mod.rs`:

- `FcpHandshakeRequest` — JSON-RPC 2.0 request with `HandshakeParams` (version, capabilities)
- `FcpHandshakeResponse` — JSON-RPC 2.0 response with `HandshakeResult` (version, negotiated capabilities)
- `HandshakeCapabilities` — lists of supported scopes and exporters
- `AnvilContext` — schema with `schema_version`, runtimes, config, diagnostics, workspace, environment, secrets_metadata
- `ContextOptions` — scopes, excludes, workspace_root, cache_dir, active_profile

### 6. Journal Format — NDJSON

Events appended as line-delimited JSON to `.anvil/journal.jsonl`:

```json
{"timestamp":"2026-07-01T12:00:00Z","operation_id":"op-123","runtime":"node","phase":"download","status":"Finished","message":"Downloaded 12.3 MB"}
```

### 7. Diagnostic Protocol

Stable types from `crates/anvil-core/src/diagnostics/`:

| Symbol | Role |
|--------|------|
| `DiagnosticContext` | Input: workspace_root, cache_dir, mode, active_profile |
| `DiagnosticMode` | Enum: `Fast`, `Deep` |
| `Severity` | Enum: `INFO`, `WARNING`, `ERROR`, `CRITICAL` |
| `Finding` | Code, severity, confidence, message, suggested quick fix |
| `QuickFix` | Description, action, target |
| `QuickFixAction` | Enum: `Repair`, `UpdateConfig`, `Install`, `Reinstall`, `ClearCache`, `SetEnv`, `RemoveFile` |
| `DiagnosticReport` | Mode, findings, health_score, elapsed_ms |
| `HealthCheck` | Trait for pluggable checks |
| `calculate_health_score` | fn: weights findings into 0-100 score |
| `DiagnosticEngine` | Runs all checks, produces report |

### 8. Secrets Engine

Stable interfaces and types from `crates/anvil-core/src/secrets/`:

| Symbol | Role |
|--------|------|
| `SecretProvider` | Trait: `set`, `get`, `delete`, `list` |
| `ConfigurationProvider` | Trait: `get_config` |
| `ValueSource` | Enum: `Explicit`, `Config`, `Profile`, `DefaultValue`, `Resolved` |
| `VarMetadata` | Value + source provenance |
| `ResolvedEnvironment` | vars + metadata |
| `KeyringSecretProvider` | OS keyring-backed |
| `FallbackSecretProvider` | Encrypted file-backed |
| `EncryptedPayload` | AES-GCM + Argon2id format |

### 9. EventBus

From `crates/anvil-core/src/event_bus/`:

| Symbol | Role |
|--------|------|
| `EventBus` | `tokio::sync::broadcast`-based publish/subscribe |
| `EventBus::new` | `(capacity: usize) -> Self` |
| `EventBus::publish` | `(event: Event)` — non-blocking send |
| `EventBus::subscribe` | `() -> Receiver<Event>` |

### 10. Operations Trait

From `crates/anvil-core/src/operations/`:

| Symbol | Role |
|--------|------|
| `Operation` trait | `name()`, `plan()`, `execute()` — the atomic unit of work |
| `Plan` trait | `to_json()`, `as_any()` — introspection |
| `Context` | Shared operation context: workspace_root, cache_dir, event_bus, config, lockfile |
| `SimplePlan` | Default plan implementation |
| `SyncPlan` | Sync-specific plan |
| `RepairPlan` | Repair-specific plan |
| Operation structs | `InitOperation`, `ResolveOperation`, `LockOperation`, `GcOperation`, `RunOperation`, `ShellOperation`, `PlanOperation`, `ValidateOperation`, `SyncOperation`, `RepairOperation`, `CleanOperation` |

### 11. Context Engine

From `crates/anvil-core/src/context/`:

| Symbol | Role |
|--------|------|
| `ContextProvider` trait | `name()`, `collect()` |
| `ContextExporter` trait | `name()`, `export()` |
| `AgentAdapter` trait | `name()`, `adapt()` |
| `ContextEngine` | Provider registry + query dispatch |
| Built-in providers | `RuntimeProviderImpl`, `ConfigurationProviderImpl`, `DiagnosticsProviderImpl`, `WorkspaceProviderImpl`, `EnvironmentProviderImpl`, `SecretsProviderImpl` |
| Built-in exporters | `JsonExporter`, `MarkdownExporter`, `McpExporter` |
| Built-in adapters | `ClaudeCodeAdapter`, `GeminiCliAdapter`, `AiderAdapter`, `ContinueAdapter` |

### 12. Environment Subsystem

From `crates/anvil-core/src/environment/`:

| Symbol | Role |
|--------|------|
| `RuntimeContextProvider` trait | `workspace_root()`, `runtime_path()` |
| `find_anvil_env` | Locate anvil.env in workspace tree |
| `parse_env_file` | Parse `KEY=VALUE` lines |
| `is_secret` | Heuristic: detect secret-like keys |
| `mask_env_vars` | Replace secret values with `****` |
| `materialize_environment` | Resolve full env with profile support |

### 13. Registry Types

From `crates/anvil-core/src/registry/`:

| Symbol | Role |
|--------|------|
| `HybridRegistry` | Internal + cache-backed lookup |
| `RegistryEntry` | Name, version, platform artifacts |
| `normalize_platform` | Normalize OS names |
| `normalize_arch` | Normalize arch names |
| `detect_platform` | Detect host platform |
| `detect_arch` | Detect host architecture |

### 14. CLI Wire Protocol

The CLI (`anvil-cli`) communicates with the core **exclusively** through the `Engine` facade. No CLI command accesses `anvil-core` internals directly (except for context commands that use `ContextEngine` directly, which is intentional — the CLI is a consumer of both the Engine API and the Context Engine API).

---

## What is NOT Frozen

These components are **excluded** from the 1.0 guarantee and may change:

| Component | Reason |
|-----------|--------|
| Internal provider implementations (e.g., individual installers) | May be refactored for plugin system |
| Plugin system | Not yet built — the entire extension architecture is upcoming |
| SDK bindings (Go, Python, TypeScript) | Not yet built |
| MCP Server | Not yet built — current `McpExporter` is a format, not a server |
| IDE integrations | Not yet built |
| GUI | Not yet built |
| Cloud Sync protocol | Not yet built |
| Anvil Registry protocol (ARRS) | Not yet built — current `HybridRegistry` is internal-only |
| Anvil Bundle format | Not yet built |
| Anvil Snapshot format | Not yet built |
| Policy Engine | Not yet built — `[policy]` section not parsed |
| `anvil benchmark` | Not yet built |
| Internal test helpers and mocks | Not stable by definition |
| `crates/anvil-drivers/` | Implementation detail of command execution |
| `crates/anvil-shim/` | Utility binary — its CLI interface is stable, internal structure is not |

---

## Deprecations

### Deprecated but Still Supported

These work today but will be removed in a future major version:

| Item | Replacement | Removal Target |
|------|-------------|----------------|
| `AiCommands::Context` (CLI) | `anvil context --format claude` | Core 2.0 |
| `AiCommands::Doctor` (CLI) | `anvil doctor --json` | Core 2.0 |
| `Engine::trace_operation` (alias) | `Engine::trace` | Core 2.0 |
| `RuntimeDetail` type alias | `RuntimeExplanation` | Core 2.0 |

### Removed Immediately

| Item | Reason |
|------|--------|
| `about = "Sovereign runtime sandbox & orchestration engine"` | Replaced with platform narrative |

---

## Stability Covenant

1. **No breaking changes to the frozen surface** without a major version bump (`2.0`).
2. **Breaking means**: removing or renaming a public symbol, changing a method signature, changing serialization format, changing behavior in a way that breaks existing callers.
3. **Additions are always allowed**: new methods, new types, new optional fields — as long as existing code continues to compile and produce the same results.
4. **Deprecation notice**: any removal from the frozen surface must be deprecated for **at least one minor version** before removal.
5. **Semantic versioning**: `MAJOR.MINOR.PATCH` — MAJOR for breaking changes, MINOR for new features (non-breaking), PATCH for bug fixes.
6. **Engram architecture records**: all changes to the frozen surface must be documented in Engram with `type: architecture` and `topic_key: architecture/core-1.0-surface`.

---

## Governance

- The frozen surface can only be modified via **architecture decision record** (ADR) with full rationale.
- Breaking change proposals require review and sign-off.
- The `anvil-core` crate is the authoritative source of truth for what is frozen. This document is a living companion — discrepancies between this document and the code are resolved in favor of the code.

---

## Sign-off

This freeze is adopted based on the architectural review of all 8 completed phases. The core is mature, tested (44 tests passing), and ready to serve as a stable foundation for the ecosystem.

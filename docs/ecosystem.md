# Forge Ecosystem Vision

Everything prior to this document was **internal** — building the core engine that makes Forge work. Everything from here is **external** — building the ecosystem that makes Forge *matter*.

The core is frozen at 1.0. It will not grow. What grows is the layer around it.

---

## Phase 9 — Plugin System

Not because Forge needs plugins today. Because it needs to grow *without modifying the core*.

### Extension Types (all trait-based)

| Category | Trait | Purpose |
|----------|-------|---------|
| **Runtime Providers** | `RuntimeProvider` | New runtime sources (custom registries, local paths, Docker) |
| **Configuration Providers** | `ConfigurationProvider` | New config sources (consul, vault, env vars from external systems) |
| **Context Providers** | `ContextProvider` | New data sources for the Context Engine (git info, CI metadata) |
| **Diagnostic Checks** | `DiagnosticCheck` | Custom health checks (license expiry, cert validity, disk space) |
| **CLI Commands** | `CliCommand` | Third-party commands registered at startup (no recompilation) |
| **Exporters** | `ContextExporter` | New output formats (YAML, TOML, HTML, DOT graph) |
| **Operations** | `Operation` | New lifecycle operations (backup, migrate, snapshot) |

### Design Constraints

- Plugins are **Rust trait objects loaded at compile time** (not WASM or dynamic linking — at least initially).
- The plugin registry lives in `forge-core` and is populated at startup.
- Each plugin has a `name`, `version`, and `enabled` flag.
- Ordering and dependency resolution between plugins is explicit.

---

## Phase 10 — Official SDK

Not just Rust. Forge's public API is frozen, which means anyone can write a binding. But official SDKs ensure quality.

### Target Languages

| Language | Priority | Use Case |
|----------|----------|----------|
| **Rust** | 1 | Native, fastest path, full access |
| **Go** | 2 | CI tooling, platform engineering |
| **Python** | 3 | Data science, ML, script integration |
| **TypeScript** | 4 | Web tooling, MCP server, IDE plugins |

### SDK Surface

Every SDK exposes the same operations:

- `ForgeEngine` — load a project, query status, run operations
- `ContextQuery` — fetch structured project context via FCP
- `Diagnostics` — run health checks, get findings
- `Environment` — resolve and manipulate environment variables
- `Secrets` — manage encrypted credentials

All SDKs communicate through the **same JSON-RPC transport** (not FFI), keeping bindings thin and maintainable.

---

## Phase 11 — MCP Server

Not a proof of concept. A product.

The MCP (Model Context Protocol) server exposes Forge's entire context engine through the standard MCP interface:

### Resources

| Resource | Description |
|----------|-------------|
| `forge://context` | Full project context (runtimes, config, diagnostics, workspace) |
| `forge://status` | Current lifecycle state |
| `forge://diagnostics` | Latest health report |
| `forge://history` | Recent operation history |

### Tools

| Tool | Description |
|------|-------------|
| `forge_run` | Execute a command in the forge environment |
| `forge_shell` | Spawn a subshell |
| `forge_sync` | Sync runtimes |
| `forge_plan` | Preview what would change |
| `forge_explain` | Deep-dive into a specific runtime |
| `forge_doctor` | Run diagnostics |

### Prompts

| Prompt | Description |
|--------|-------------|
| `forge:status` | "Summarize the current forge environment state" |
| `forge:diagnose` | "Diagnose issues in this project" |
| `forge:explain` | "Explain how {runtime} is configured" |

### Notifications

| Notification | Description |
|-------------|-------------|
| `forge/state_changed` | Lifecycle state transition |
| `forge/error` | Operation failure |
| `forge/warning` | Health degradation detected |

---

## Phase 12 — IDE Integration

Every IDE gets access to the **same Context Engine** through the same protocol.

| IDE | Integration | Key Feature |
|-----|-------------|-------------|
| **Zed** | Extension | Status bar indicator, runtime switcher, inline diagnostics |
| **VS Code** | Extension | Environment viewer, problem matcher, context panel |
| **Neovim** | Plugin | `:ForgeStatus`, `:ForgeDoctor`, LSP-like diagnostics |
| **JetBrains** | Plugin | Tool window, run configuration integration, project settings |

The integration pattern is always the same:

1. IDE starts → MCP client connects to Forge
2. Forge provides context via `forge://context`
3. IDE renders environment state in its native UI
4. Diagnostics appear as IDE-native problems
5. User can switch runtimes, inspect config, and run commands without leaving the editor

---

## Phase 13 — GUI

Not to replace the CLI. To *visualize* what the CLI already surfaces.

### Dashboard Views

| View | Data Source |
|------|-------------|
| **Runtime** | Installed runtimes, versions, cache status, shim health |
| **Configuration** | Active `forge.toml`, resolved profiles, env vars |
| **Diagnostics** | Health score, findings timeline, repair history |
| **History** | Operation timeline, durations, nested traces |
| **Events** | Live event stream, filtered by severity |
| **Secrets** | Keyring status, masked metadata, import/export |
| **Profiles** | Active profile, available profiles, variable diff |
| **Context** | Full FCP context rendered as structured data |

### Tech Stack

- **Tauri** (Rust backend + web frontend)
- Reuses the same `Engine` facade as the CLI
- Zero additional runtime dependencies

---

---

## Forge Registry

Not a package registry. A **toolchain registry**.

```text
registry.forge.sh
    └── python/
        └── 3.13.0/
            ├── linux-x86_64.tar.gz
            ├── macos-arm64.tar.gz
            ├── windows-x86_64.zip
            ├── sha256sums.txt
            ├── mirrors.json        # fallback download mirrors
            └── metadata.toml       # deps, build info, license
    └── llvm/
    └── android-sdk/
    └── cuda/
    └── jdk/
```

### Key Differences from Package Registries

| Aspect | Package Registry | Toolchain Registry |
|--------|-----------------|-------------------|
| Content | Libraries, apps | Language runtimes, SDKs, compilers |
| Granularity | Version-patch | Full platform-artifact matrix |
| Integrity | Package signatures | Checksums + GPG signatures |
| Distribution | Single source | Mirrors + CDN |
| Metadata | Dependencies | Platforms, hashes, system reqs |

The **Forge Runtime Registry Specification (FRRS)** defines an open format so anyone can host a compatible registry.

---

## Forge Bundle

A single-file distribution format:

```bash
# Package an entire project
forge bundle
# -> project.forge (manifest + lock + context + metadata + checksums)

# Restore anywhere
forge restore project.forge
# -> forge.toml, forge.lock, .forge/
```

The `.forge` extension file is the **universal handoff artifact** — send it to a teammate, an agent, a CI pipeline, or deploy it as an immutable environment descriptor.

---

## Forge Snapshot

Save and restore full environment state:

```bash
# Capture everything
forge snapshot
# -> .forge/snapshots/2026-07-01T12-00-00/

# Roll back to a known-good state
forge restore snapshot 2026-07-01T12-00-00
```

Snapshots include lockfile state, cache metadata, profile configuration, and journal history — not the binaries themselves (those are re-downloaded or pulled from cache).

---

## Forge Benchmark

Measure what matters:

```bash
forge benchmark

# Results:
#   Sync time      : 1.2s
#   Diagnostic time: 0.3s
#   Context time    : 0.05s
#   Launch time     : 0.08s
#   Health score    : 97/100
```

Benchmarks are deterministic and comparable across machines — useful for CI gates and regression detection.

---

## Forge Explain Everything

Forge already has `forge explain` for runtimes. Extend it to every domain:

```bash
forge explain runtime     # Runtime configuration, path, version, state
forge explain operation   # What an operation did, why, how long
forge explain context     # What context was collected, what was masked
forge explain config      # Resolved configuration with provenance
forge explain profile     # Active profile, variables, precedence chain
```

Each explain command returns a structured, human-readable breakdown with traceability to the source of truth.

---

## Forge Policy Engine

Declarative policies that gate operation execution:

```toml
[policy]
allow_network = false
require_hashes = true
forbid_unlocked = true
minimum_health = 95
require_profiles = ["production"]
```

When a `forge run` or `forge up` is executed, the policy engine validates the request before any operation starts. Violations are reported with explanations and suggested remediations.

---

## Open Specifications

Forge defines three public specifications for interoperability:

### 1. Forge Context Protocol (FCP)

A versioned, schema-stable protocol for sharing development context.

- **Transport:** JSON-RPC 2.0
- **Core operation:** `forge.context.query` → `ForgeContext`
- **Handshake:** Capability negotiation (`FcpHandshakeRequest`/`FcpHandshakeResponse`)
- **Exporters:** JSON, Markdown, MCP
- **Agent adapters:** Claude Code, Gemini CLI, Aider, Continue

Anyone can implement an FCP client or server. The schema is versioned and frozen.

### 2. Forge Manifest Specification (FMS)

The formal specification for `forge.toml`, `forge.lock`, profiles, and environment files.

- **`forge.toml`:** Runtime declarations, profile definitions, policy rules
- **`forge.lock`:** Resolved runtime entries with checksums and platform bindings
- **`forge.env`:** Declarative environment variable definitions with source annotations
- **Profiles:** Named variable sets with precedence rules and inheritance

### 3. Forge Runtime Registry Specification (FRRS)

An open standard for describing and distributing runtimes.

- **Entry format:** Name, version, platform artifacts (URL, size, hash)
- **Optional metadata:** Mirrors, signatures, dependencies, system requirements
- **Resolution order:** Local cache → configured registries → fallback mirrors
- **Compatibility:** Any registry format that produces valid `RegistryEntry` structs

---

## Roadmap Summary

```text
Core 1.0 ────────────────────────────────► Frozen (today)
                                              │
Phase  9: Plugin System                      │  Q3 2026
Phase 10: Official SDK (Rust → Go → Py → TS) │  Q3-Q4 2026
Phase 11: MCP Server                         │  Q4 2026
Phase 12: IDE Integration                    │  Q1 2027
Phase 13: GUI                                │  Q1 2027
Forge Registry                               │  Ongoing
Public Specifications (FCP, FMS, FRRS)       │  H2 2026
```

These are not commitments — they are a map. The priority at any given time should reflect what best serves the project's adoption and ecosystem growth.

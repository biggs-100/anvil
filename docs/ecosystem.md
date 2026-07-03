# Anvil Ecosystem Vision

Everything prior to this document was **internal** — building the core engine that makes Anvil work. Everything from here is **external** — building the ecosystem that makes Anvil *matter*.

The core is frozen at 1.0. It will not grow. What grows is the layer around it.

---

## Phase 9 — Plugin System

Not because Anvil needs plugins today. Because it needs to grow *without modifying the core*.

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
- The plugin registry lives in `anvil-core` and is populated at startup.
- Each plugin has a `name`, `version`, and `enabled` flag.
- Ordering and dependency resolution between plugins is explicit.

---

## Phase 10 — Official SDK

Not just Rust. Anvil's public API is frozen, which means anyone can write a binding. But official SDKs ensure quality.

### Target Languages

| Language | Priority | Use Case |
|----------|----------|----------|
| **Rust** | 1 | Native, fastest path, full access |
| **Go** | 2 | CI tooling, platform engineering |
| **Python** | 3 | Data science, ML, script integration |
| **TypeScript** | 4 | Web tooling, MCP server, IDE plugins |

### SDK Surface

Every SDK exposes the same operations:

- `Engine` — load a project, query status, run operations
- `ContextQuery` — fetch structured project context via ACP
- `Diagnostics` — run health checks, get findings
- `Environment` — resolve and manipulate environment variables
- `Secrets` — manage encrypted credentials

All SDKs communicate through the **same JSON-RPC transport** (not FFI), keeping bindings thin and maintainable.

---

## Phase 11 — MCP Server

Not a proof of concept. A product.

The MCP (Model Context Protocol) server exposes Anvil's entire context engine through the standard MCP interface:

### Resources

| Resource | Description |
|----------|-------------|
| `anvil://context` | Full project context (runtimes, config, diagnostics, workspace) |
| `anvil://status` | Current lifecycle state |
| `anvil://diagnostics` | Latest health report |
| `anvil://history` | Recent operation history |

### Tools

| Tool | Description |
|------|-------------|
| `anvil_run` | Execute a command in the anvil environment |
| `anvil_shell` | Spawn a subshell |
| `anvil_sync` | Sync runtimes |
| `anvil_plan` | Preview what would change |
| `anvil_explain` | Deep-dive into a specific runtime |
| `anvil_doctor` | Run diagnostics |

### Prompts

| Prompt | Description |
|--------|-------------|
| `anvil:status` | "Summarize the current anvil environment state" |
| `anvil:diagnose` | "Diagnose issues in this project" |
| `anvil:explain` | "Explain how {runtime} is configured" |

### Notifications

| Notification | Description |
|-------------|-------------|
| `anvil/state_changed` | Lifecycle state transition |
| `anvil/error` | Operation failure |
| `anvil/warning` | Health degradation detected |

---

## Phase 12 — IDE Integration

Every IDE gets access to the **same Context Engine** through the same protocol.

| IDE | Integration | Key Feature |
|-----|-------------|-------------|
| **Zed** | Extension | Status bar indicator, runtime switcher, inline diagnostics |
| **VS Code** | Extension | Environment viewer, problem matcher, context panel |
| **Neovim** | Plugin | `:AnvilStatus`, `:AnvilDoctor`, LSP-like diagnostics |
| **JetBrains** | Plugin | Tool window, run configuration integration, project settings |

The integration pattern is always the same:

1. IDE starts → MCP client connects to Anvil
2. Anvil provides context via `anvil://context`
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
| **Configuration** | Active `anvil.toml`, resolved profiles, env vars |
| **Diagnostics** | Health score, findings timeline, repair history |
| **History** | Operation timeline, durations, nested traces |
| **Events** | Live event stream, filtered by severity |
| **Secrets** | Keyring status, masked metadata, import/export |
| **Profiles** | Active profile, available profiles, variable diff |
| **Context** | Full ACP context rendered as structured data |

### Tech Stack

- **Tauri** (Rust backend + web frontend)
- Reuses the same `Engine` facade as the CLI
- Zero additional runtime dependencies

---

---

## Anvil Registry

Not a package registry. A **toolchain registry**.

```text
registry.anvil.dev
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

The **Anvil Runtime Registry Specification (ARRS)** defines an open format so anyone can host a compatible registry.

---

## Anvil Bundle

A single-file distribution format:

```bash
# Package an entire project
anvil bundle
# -> project.anvil (manifest + lock + context + metadata + checksums)

# Restore anywhere
anvil restore project.anvil
# -> anvil.toml, anvil.lock, .anvil/
```

The `.anvil` extension file is the **universal handoff artifact** — send it to a teammate, an agent, a CI pipeline, or deploy it as an immutable environment descriptor.

---

## Anvil Snapshot

Save and restore full environment state:

```bash
# Capture everything
anvil snapshot
# -> .anvil/snapshots/2026-07-01T12-00-00/

# Roll back to a known-good state
anvil restore snapshot 2026-07-01T12-00-00
```

Snapshots include lockfile state, cache metadata, profile configuration, and journal history — not the binaries themselves (those are re-downloaded or pulled from cache).

---

## Anvil Benchmark

Measure what matters:

```bash
anvil benchmark

# Results:
#   Sync time      : 1.2s
#   Diagnostic time: 0.3s
#   Context time    : 0.05s
#   Launch time     : 0.08s
#   Health score    : 97/100
```

Benchmarks are deterministic and comparable across machines — useful for CI gates and regression detection.

---

## Anvil Explain Everything

Anvil already has `anvil explain` for runtimes. Extend it to every domain:

```bash
anvil explain runtime     # Runtime configuration, path, version, state
anvil explain operation   # What an operation did, why, how long
anvil explain context     # What context was collected, what was masked
anvil explain config      # Resolved configuration with provenance
anvil explain profile     # Active profile, variables, precedence chain
```

Each explain command returns a structured, human-readable breakdown with traceability to the source of truth.

---

## Anvil Policy Engine

Declarative policies that gate operation execution:

```toml
[policy]
allow_network = false
require_hashes = true
forbid_unlocked = true
minimum_health = 95
require_profiles = ["production"]
```

When a `anvil run` or `anvil up` is executed, the policy engine validates the request before any operation starts. Violations are reported with explanations and suggested remediations.

---

## Open Specifications

Anvil defines three public specifications for interoperability:

### 1. Anvil Context Protocol (ACP)

A versioned, schema-stable protocol for sharing development context.

- **Transport:** JSON-RPC 2.0
- **Core operation:** `anvil.context.query` → `AnvilContext`
- **Handshake:** Capability negotiation (`FcpHandshakeRequest`/`FcpHandshakeResponse`)
- **Exporters:** JSON, Markdown, MCP
- **Agent adapters:** Claude Code, Gemini CLI, Aider, Continue

Anyone can implement an ACP client or server. The schema is versioned and frozen.

### 2. Anvil Manifest Specification (AMS)

The formal specification for `anvil.toml`, `anvil.lock`, profiles, and environment files.

- **`anvil.toml`:** Runtime declarations, profile definitions, policy rules
- **`anvil.lock`:** Resolved runtime entries with checksums and platform bindings
- **`anvil.env`:** Declarative environment variable definitions with source annotations
- **Profiles:** Named variable sets with precedence rules and inheritance

### 3. Anvil Runtime Registry Specification (ARRS)

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
Anvil Registry                               │  Ongoing
Public Specifications (ACP, AMS, ARRS)       │  H2 2026
```

These are not commitments — they are a map. The priority at any given time should reflect what best serves the project's adoption and ecosystem growth.

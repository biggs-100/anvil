# Forge

**Create, run, inspect, and share reproducible development environments — for humans, tools, and AI agents.**

Forge is a platform that treats development environments as first-class, reproducible artifacts. Not a package manager. Not a container runtime. A **context platform** that manages runtimes, configuration, secrets, diagnostics, and environment state through a unified engine and a stable public API.

---

## Why Forge?

Development environments today are fragile. A `~/.bashrc` here, a Homebrew install there, a Dockerfile that worked last week, a Node version manager that doesn't know about your Python — and forget about sharing any of it with an AI agent.

Forge solves this by providing:

- **Reproducible runtime toolchains** — declare `node = "20.11.0"` in `forge.toml`, get the exact binary with verified checksums.
- **Unified configuration and secrets** — environment variables, profiles, and encrypted secrets managed through a single engine.
- **Built-in diagnostics** — health checks, repair planning, and explainability for every component.
- **Context extraction** — a stable protocol (FCP) that surfaces project state to humans, CLI tools, IDEs, and AI agents in a structured, secure format.
- **No lock-in** — everything works locally, offline-first, with no daemon, no registry dependency, and no vendor.

---

## Quick Start

```bash
# Install (one binary)
curl -fsSL https://forge.sh/install | sh

# Initialize in your project
cd my-project
forge init

# Declare runtimes in forge.toml
cat >> forge.toml <<EOF
[runtimes]
node = "20.11.0"
python = "3.12.0"
EOF

# Resolve and download
forge up

# Use it
forge run node --version   # -> v20.11.0
forge run python --version # -> Python 3.12.0

# Spawn a subshell in the isolated environment
forge shell
```

---

## Architecture

Forge's architecture is organized into **stable core** (frozen at 1.0) and **extensible ecosystem** layers.

### Core (Stable — frozen at v1.0)

| Component | Status | Role |
|-----------|--------|------|
| **Runtime Engine** | ✅ Stable | Resolve, download, verify checksums, extract, cache, and shim any runtime |
| **Lifecycle Engine** | ✅ Stable | State machine: uninitialized → ready, with plan/apply/rollback semantics |
| **Operations Layer** | ✅ Stable | Atomic operations (init, resolve, lock, sync, install, clean, gc) with transaction safety |
| **Configuration Engine** | ✅ Stable | Parse `forge.toml`, resolve profiles, interpolate environment variables |
| **Secrets Engine** | ✅ Stable | OS keyring with local fallback, AES-GCM encryption, import/export |
| **Diagnostic Engine** | ✅ Stable | Health checks, repair planner, explainability, quick-fix suggestions |
| **Context Engine** | ✅ Stable | Aggregate runtimes, config, diagnostics, workspace, env, secrets metadata into a unified schema |
| **Observability** | ✅ Stable | Event bus, journal (NDJSON), trace trees, history queries, live event streaming |
| **Public API (v1)** | ✅ Stable | `Engine` facade — the only entry point for CLI, SDK, and integrations |

### Ecosystem (Complete)

Beyond the stable core, Forge's ecosystem is fully built:

| Component | Status | Description |
|-----------|--------|-------------|
| **Plugin System** | ✅ Complete | Trait-based extensions for providers, exporters, diagnostics, and CLI commands |
| **SDK** | ✅ Complete | Official bindings for Rust, Go, Python, TypeScript |
| **MCP Server** | ✅ Complete | Full Model Context Protocol with resources, tools, prompts, notifications |
| **IDE Integration** | ✅ Complete | VS Code extension + Neovim plugin (MCP-based) |
| **TUI** | ✅ Complete | Terminal dashboard with Ratatui (4 views, keyboard-driven) |
| **Bundle** | ✅ Complete | `forge bundle` — deterministic tar+gzip with SHA-256 verification |
| **Snapshot** | ✅ Complete | `forge snapshot` — save/restore full environment state |
| **Policy Engine** | ✅ Complete | Declarative pre-flight rules (network, hashes, health, profiles) |
| **Explain Everything** | ✅ Complete | 5 explain subcommands (runtime, operation, context, config, profile) |
| **Benchmark** | ✅ Complete | 5 performance metrics with table and JSON output |
| **Forge Registry** | ✅ Complete | FRRS open format + remote registry client |
| **Public Specs** | ✅ Complete | FCP, FMS, FRRS — open, versioned, frozen specifications |

---

## CLI Reference

```text
# Core lifecycle
forge init      Initialize forge in the current directory
forge resolve   Resolve runtime versions
forge lock      Generate or update forge.lock
forge sync      Sync runtimes from lockfile
forge up        Resolve + lock + sync (all-in-one)
forge run       Execute a command inside the activated environment
forge shell     Spawn an interactive subshell
forge status    Show lifecycle status
forge inspect   Inspect environment health
forge repair    Repair corrupted or missing runtimes
forge plan      Show proposed changes plan
forge clean     Clean local cache
forge gc        Garbage collect unused runtimes

# Context & diagnostics
forge context   Export project context (JSON, Markdown, MCP, agent-adapted)
forge doctor    Run diagnostics and health checks
forge explain   Explain runtime, operation, context, config, or profile
forge which     Locate a runtime binary

# Observability
forge history   Show past operations
forge trace     Show operation hierarchy and durations
forge events    Stream live operation events

# Configuration & secrets
forge env       Manage environment variables
forge secret    Manage secure credentials

# Distribution
forge bundle    Package project into a portable .forge archive
forge restore   Restore a project from a .forge archive

# State management
forge snapshot  Save current environment state
forge restore   Restore environment to a previous snapshot

# AI & tooling
forge ai        AI-agent-specific context and diagnostics
forge mcp       MCP server (Model Context Protocol) over stdio
forge jsonrpc   JSON-RPC 2.0 server for SDK integration

# Advanced
forge tui       Terminal dashboard (Ratatui)
forge benchmark Performance benchmarks
forge registry  Refresh remote registry cache
```

---

## Public Specifications

Forge is more than an implementation. It defines three open specifications:

| Spec | Description | Document |
|------|-------------|----------|
| **FCP** | Forge Context Protocol v1 — JSON-RPC protocol for extracting dev environment context | [`docs/specs/fcp-spec.md`](docs/specs/fcp-spec.md) |
| **FMS** | Forge Manifest Specification v1 — forge.toml, forge.lock, profiles, precedence | [`docs/specs/fms-spec.md`](docs/specs/fms-spec.md) |
| **FRRS** | Forge Runtime Registry Specification — open format for toolchain metadata | `openspec/specs/frrs-spec/spec.md` |

These specifications are designed to enable interoperability without reimplementing the core.

---

## Development

```bash
cargo build
cargo test
cargo run -- help
```

Forge is a Rust workspace with six crates:

| Crate | Role |
|-------|------|
| `forge-core` | Engine — all core logic, traits, providers, API |
| `forge-cli` | CLI — command parsing and user-facing interface |
| `forge-sdk` | Official Rust SDK (typed Engine wrapper) |
| `forge-tui` | Terminal dashboard (Ratatui) |
| `forge-drivers` | Standard command runners |
| `forge-shim` | Runtime shim binary (<5ms overhead) |

### SDKs & Integrations

| Language | Location | Type |
|----------|----------|------|
| **Go** | `sdks/go/` | JSON-RPC thin client |
| **Python** | `sdks/python/` | JSON-RPC thin client |
| **TypeScript** | `sdks/typescript/` | JSON-RPC thin client |
| **VS Code** | `extensions/vscode/` | MCP-based extension |
| **Neovim** | `extensions/neovim/` | MCP-based plugin |

---

## License

MIT

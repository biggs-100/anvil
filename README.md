# Anvil

[![CI](https://github.com/biggs-100/anvil/actions/workflows/ci.yml/badge.svg)](https://github.com/biggs-100/anvil/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**Create, run, inspect, and share reproducible development environments — for humans, tools, and AI agents.**

Anvil is a platform that treats development environments as first-class, reproducible artifacts. Not a package manager. Not a container runtime. A **context platform** that manages runtimes, configuration, secrets, diagnostics, and environment state through a unified engine and a stable public API.

---

## Why Anvil?

Development environments today are fragile. A `~/.bashrc` here, a Homebrew install there, a Dockerfile that worked last week, a Node version manager that doesn't know about your Python — and forget about sharing any of it with an AI agent.

Anvil solves this by providing:

- **Reproducible runtime toolchains** — declare `node = "20.11.0"` in `anvil.toml`, get the exact binary with verified checksums.
- **Unified configuration and secrets** — environment variables, profiles, and encrypted secrets managed through a single engine.
- **Built-in diagnostics** — health checks, repair planning, and explainability for every component.
- **Context extraction** — a stable protocol (ACP) that surfaces project state to humans, CLI tools, IDEs, and AI agents in a structured, secure format.
- **No lock-in** — everything works locally, offline-first, with no daemon, no registry dependency, and no vendor.

---

## Quick Start

```bash
# Install from source (Rust required)
cargo install --git https://github.com/biggs-100/anvil.git anvil-cli

# Or clone and build
git clone https://github.com/biggs-100/anvil.git
cd anvil
cargo build --release
./target/release/anvil-cli --help

# Initialize in your project
cd my-project
anvil init

# Declare runtimes in anvil.toml
cat >> anvil.toml <<EOF
[runtimes]
node = "20.11.0"
python = "3.12.0"
EOF

# Resolve and download
anvil up

# Use it
anvil run node --version   # -> v20.11.0
anvil run python --version # -> Python 3.12.0

# Spawn a subshell in the isolated environment
anvil shell
```

---

## Architecture

Anvil's architecture is organized into **stable core** (frozen at 1.0) and **extensible ecosystem** layers.

### Core (Stable — frozen at v1.0)

| Component | Status | Role |
|-----------|--------|------|
| **Runtime Engine** | ✅ Stable | Resolve, download, verify checksums, extract, cache, and shim any runtime |
| **Lifecycle Engine** | ✅ Stable | State machine: uninitialized → ready, with plan/apply/rollback semantics |
| **Operations Layer** | ✅ Stable | Atomic operations (init, resolve, lock, sync, install, clean, gc) with transaction safety |
| **Configuration Engine** | ✅ Stable | Parse `anvil.toml`, resolve profiles, interpolate environment variables |
| **Secrets Engine** | ✅ Stable | OS keyring with local fallback, AES-GCM encryption, import/export |
| **Diagnostic Engine** | ✅ Stable | Health checks, repair planner, explainability, quick-fix suggestions |
| **Context Engine** | ✅ Stable | Aggregate runtimes, config, diagnostics, workspace, env, secrets metadata into a unified schema |
| **Observability** | ✅ Stable | Event bus, journal (NDJSON), trace trees, history queries, live event streaming |
| **Public API (v1)** | ✅ Stable | `Engine` facade — the only entry point for CLI, SDK, and integrations |

### Ecosystem (Complete)

Beyond the stable core, Anvil's ecosystem is fully built:

| Component | Status | Description |
|-----------|--------|-------------|
| **Plugin System** | ✅ Complete | Trait-based extensions for providers, exporters, diagnostics, and CLI commands |
| **SDK** | ✅ Complete | Official bindings for Rust, Go, Python, TypeScript |
| **MCP Server** | ✅ Complete | Full Model Context Protocol with resources, tools, prompts, notifications |
| **IDE Integration** | ✅ Complete | VS Code extension + Neovim plugin (MCP-based) |
| **TUI** | ✅ Complete | Terminal dashboard with Ratatui (4 views, keyboard-driven) |
| **Bundle** | ✅ Complete | `anvil bundle` — deterministic tar+gzip with SHA-256 verification |
| **Snapshot** | ✅ Complete | `anvil snapshot` — save/restore full environment state |
| **Policy Engine** | ✅ Complete | Declarative pre-flight rules (network, hashes, health, profiles) |
| **Explain Everything** | ✅ Complete | 5 explain subcommands (runtime, operation, context, config, profile) |
| **Benchmark** | ✅ Complete | 5 performance metrics with table and JSON output |
| **Registry** | ✅ Complete | ARRS open format + remote registry client |
| **Public Specs** | ✅ Complete | ACP, AMS, ARRS — open, versioned, frozen specifications |
| **Supply Chain Security** | ✅ Complete | GPG registry signing, pin-by-hash, anvil audit, SHA-256 verification |

---

## CLI Reference

```text
# Core lifecycle
anvil init      Initialize anvil in the current directory
anvil resolve   Resolve runtime versions
anvil lock      Generate or update anvil.lock
anvil sync      Sync runtimes from lockfile
anvil up        Resolve + lock + sync (all-in-one)
anvil run       Execute a command inside the activated environment
anvil shell     Spawn an interactive subshell
anvil status    Show lifecycle status
anvil inspect   Inspect environment health
anvil repair    Repair corrupted or missing runtimes
anvil plan      Show proposed changes plan
anvil clean     Clean local cache
anvil gc        Garbage collect unused runtimes

# Context & diagnostics
anvil context   Export project context (JSON, Markdown, MCP, agent-adapted)
anvil doctor    Run diagnostics and health checks
anvil explain   Explain subcommands: runtime, operation, context, config, profile
anvil which     Locate a runtime binary

# Supply chain security
anvil audit     Show download/install history with checksums

# Observability
anvil history   Show past operations
anvil trace     Show operation hierarchy and durations
anvil events    Stream live operation events

# Configuration & secrets
anvil env       Manage environment variables
anvil secret    Manage secure credentials

# Distribution
anvil bundle           Package project into a portable .anvil archive
anvil bundle restore   Restore a project from a .anvil archive

# State management
anvil snapshot                Save current environment state
anvil snapshot list           List available snapshots
anvil snapshot restore        Restore environment from a snapshot

# AI & tooling
anvil ai        AI-agent-specific context and diagnostics
anvil mcp       MCP server (Model Context Protocol) over stdio
anvil jsonrpc   JSON-RPC 2.0 server for SDK integration

# Advanced
anvil tui       Terminal dashboard (Ratatui)
anvil benchmark Performance benchmarks
anvil registry  Refresh remote registry cache
```

---

## Public Specifications

Anvil is more than an implementation. It defines three open specifications:

| Spec | Description | Document |
|------|-------------|----------|
| **ACP** | Anvil Context Protocol v1 — JSON-RPC protocol for extracting dev environment context | [`docs/specs/acp-spec.md`](docs/specs/acp-spec.md) |
| **AMS** | Anvil Manifest Specification v1 — anvil.toml, anvil.lock, profiles, precedence | [`docs/specs/ams-spec.md`](docs/specs/ams-spec.md) |
| **ARRS** | Anvil Runtime Registry Specification — open format for toolchain metadata | `openspec/specs/arrs-spec/spec.md` |

These specifications are designed to enable interoperability without reimplementing the core.

---

## Development

```bash
cargo build
cargo test
cargo run -- help
```

Anvil is a Rust workspace with six crates:

| Crate | Role |
|-------|------|
| `anvil-core` | Engine — all core logic, traits, providers, API |
| `anvil-cli` | CLI — command parsing and user-facing interface |
| `anvil-sdk` | Official Rust SDK (typed Engine wrapper) |
| `anvil-tui` | Terminal dashboard (Ratatui) |
| `anvil-drivers` | Standard command runners |
| `anvil-shim` | Runtime shim binary (<5ms overhead) |

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

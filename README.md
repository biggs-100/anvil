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

### Ecosystem (In Progress)

Beyond the core, Forge is designed for extension:

- **Plugin System** (Phase 9) — trait-based extensions for providers, exporters, diagnostics, and CLI commands
- **SDK** (Phase 10) — official bindings for Rust, Go, Python, TypeScript
- **MCP Server** (Phase 11) — full Model Context Protocol implementation
- **IDE Integration** (Phase 12) — Zed, VS Code, Neovim, JetBrains
- **GUI** (Phase 13) — visual dashboard for runtime, config, diagnostics, events
- **Cloud Sync** (Phase 14) — publish and share manifests, locks, and profiles
- **Forge Registry** — a public registry of toolchains, not packages
- **Forge Bundle** — single-file project distribution

---

## CLI Reference

```text
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

forge context   Export project context (JSON, Markdown, MCP, agent-adapted)
forge explain   Deep-dive into a specific runtime
forge doctor    Run diagnostics and health checks
forge which     Locate a runtime binary
forge history   Show past operations
forge trace     Show operation hierarchy and durations
forge events    Stream live operation events

forge env       Manage environment variables
forge secret    Manage secure credentials
forge ai        AI-agent-specific context and diagnostics
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

Forge is a Rust workspace with four crates:

| Crate | Role |
|-------|------|
| `forge-core` | Engine — all core logic, traits, providers, API |
| `forge-cli` | CLI — command parsing and user-facing interface |
| `forge-drivers` | Standard command runners |
| `forge-shim` | Runtime shim binary (<5ms overhead) |

---

## License

MIT

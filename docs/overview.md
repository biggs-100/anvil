# Forge Architecture Overview

**Version:** 1.0 (Core frozen)
**Last updated:** 2026-07-01

---

## What is Forge?

Forge is a **platform for creating, running, inspecting, and sharing reproducible development environments**. It serves three audiences through the same core:

| Audience | How they use Forge |
|----------|-------------------|
| **Humans** | `forge shell`, `forge run`, `forge status`, `forge explain` — daily development workflows |
| **Tools** | `forge context --format json`, `forge doctor --json`, `forge history` — CI, scripts, automation |
| **AI Agents** | `forge ai context`, `forge ai doctor`, FCP protocol — structured project understanding |

This is not a "container alternative" or "Nix replacement." The core abstraction is **reproducible context**, not package management or system configuration.

---

## Core Tenets

1. **Offline-first by design.** Forge never requires a network connection for basic operation. Registries and downloads are cached; everything works after the initial sync.
2. **No daemon, no server, no lock-in.** A single binary. No background processes. No vendor dependency.
3. **Reproducibility through content-addressed integrity.** Every runtime has verified checksums. Every environment is defined by a declarative manifest.
4. **Context is a first-class product.** Project state is not locked inside the tool — it is extracted, structured, and exported via the Forge Context Protocol.
5. **Stable core, extensible ecosystem.** The core is frozen at 1.0. All new capabilities come through plugins, SDKs, and integrations.

---

## Component Map

```
                  ┌──────────────────────┐
                  │     CLI (forge-cli)   │
                  │  commands, formatters │
                  └──────┬───────────────┘
                         │
                  ┌──────▼───────────────┐
                  │  Public API (v1)     │
                  │   Engine Facade      │
                  └──────┬───────────────┘
                         │
          ┌──────────────┼──────────────┐
          │              │              │
   ┌──────▼─────┐ ┌──────▼─────┐ ┌──────▼──────┐
   │  Operations │ │  Context   │ │  Diagnostic  │
   │   Layer     │ │   Engine   │ │   Engine     │
   │ (atomic TX) │ │   (FCP)    │ │ (health/repair)│
   └──────┬─────┘ └──────┬─────┘ └──────┬──────┘
          │              │              │
   ┌──────▼─────┐ ┌──────▼─────┐ ┌──────▼──────┐
   │  Runtime   │ │  Config &  │ │  Observability│
   │   Engine   │ │   Secrets  │ │  (EventBus,  │
   │ (resolve,  │ │  (profiles, │ │   Journal)   │
   │ install,   │ │  keyring)  │ │              │
   │ cache,     │ │            │ │              │
   │ shim)      │ │            │ │              │
   └────────────┘ └────────────┘ └──────────────┘
```

### Core Data Flow

1. **`forge.toml`** declares desired state (runtimes, profiles, env vars).
2. **Operations Layer** computes a plan, executes it atomically, and updates state.
3. **Runtime Engine** resolves versions, downloads with checksum verification, extracts, caches, and generates shims.
4. **Configuration & Secrets** resolve environment variables, apply profiles, and manage encrypted credentials.
5. **Diagnostic Engine** runs health checks, generates repair plans, and produces structured findings.
6. **Context Engine** aggregates everything into a unified `ForgeContext` schema and exports it via FCP.
7. **Observability** records every operation in the journal, streams events for live consumption, and supports trace queries.

---

## Lifecycle States

The environment state machine has eight well-defined states:

```
Uninitialized
    │
    ▼
Initialized (forge.toml exists)
    │
    ▼
Locked (forge.lock generated)
    │
    ▼
Synced (runtimes extracted)
    │
    ▼
Ready (shims cached, environment complete)
    │
    ├── Dirty (state changed after ready)
    └── Broken (integrity check failed)
```

Transitions are always planned first (`forge plan`), then executed (`forge up`, `forge repair`).

---

## Stability Guarantee (Core 1.0)

The following are frozen and stable:

- **Public API** (`Engine` facade in `crates/forge-core/src/api/v1.rs`) — all queries and operations
- **Core types** (`RuntimeId`, `Lockfile`, `LifecycleState`, `Event`, `ForgeContext`, `DiagnosticReport`)
- **Manifest format** (`forge.toml` schema, `forge.lock` schema)
- **FCP handshake protocol** (JSON-RPC schema, version negotiation, capability exchange)
- **NDJSON journal format** (`.forge/journal.jsonl`)
- **Secrets engine** (keyring integration, encrypted payload format)
- **Diagnostic protocol** (severity levels, quick-fix format, health score computation)

What is NOT frozen (expected to evolve):

- Internal provider implementations
- Plugin system (not yet built)
- SDK bindings (not yet built)
- MCP server (not yet built)
- IDE integrations (not yet built)
- GUI (not yet built)
- Registry protocol (not yet built)

---

## Key Architectural Decisions

| ADR | Decision |
|-----|----------|
| ADR-0001 | Asynchronous journal storage via background tokio task |
| ADR-0002 | Engine facade isolation — `Engine` as the only public entry point |
| ADR-0003 | In-process EventBus via `tokio::sync::broadcast` |
| ADR-0004 | CLI introspection subcommands: `history`, `explain`, `trace`, `events` |
| ADR-0005 | NDJSON (JSON Lines) for journal format under `.forge/journal.jsonl` |
| ADR-0006 | Lightweight polling for cache integrity verification |

See individual ADR files in `docs/adr/` for full details.

---

## Project Structure

```
forge/
├── Cargo.toml             # Workspace definition
├── forge.toml             # Project runtime manifest
├── forge.lock             # Resolved runtime lockfile
├── forge.env              # Environment variable definitions
├── README.md
├── docs/
│   ├── adr/               # Architecture Decision Records
│   ├── overview.md        # This file
│   └── ecosystem.md       # Ecosystem vision and roadmap
├── openspec/
│   ├── config.yaml        # SDD configuration
│   ├── specs/             # Formal specifications (stable)
│   └── changes/archive/   # Completed SDD phase artifacts
├── crates/
│   ├── forge-core/        # Core engine
│   ├── forge-cli/         # CLI
│   ├── forge-drivers/     # Standard command runners
│   └── forge-shim/        # Runtime shim binary
└── .forge/                # Local cache and state
```

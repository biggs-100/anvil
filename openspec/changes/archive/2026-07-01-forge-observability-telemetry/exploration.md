# Exploration: Observability & Architecture (Phase 5.5)

This document explores the architectural design and planning for Phase 5.5 (Observability & Architecture) of the `forge-observability-telemetry` change. It outlines how to capture domain events, implement CLI introspection commands, define a stable API facade, and document decisions through Architecture Decision Records (ADRs).

---

### 1. Capture and Persist Domain Events
To capture and persist domain events from the `EventBus` into the local Operation Journal file (`.anvil/journal.jsonl`), we will establish a structured journal logging component.

#### Implementation Design
- **Path**: `.anvil/journal.jsonl` under the workspace root.
- **Format**: JSON Lines (NDJSON), where each line is a serialized `Event` struct.
- **Mechanism**:
  - The `Context` struct contains an `EventBus` instance.
  - A thread-safe, non-blocking `JournalLogger` will subscribe to the `EventBus` at the startup of any command or operation.
  - It will receive published events via `broadcast::Receiver<Event>`.
  - To prevent filesystem blocking on the main asynchronous runtime, a background worker task will use a buffered channel to write events to `.anvil/journal.jsonl`.
  - We use standard append mode (`std::fs::OpenOptions::new().create(true).append(true).open(...)`) to ensure atomic writes.

```rust
// crates/anvil-core/src/event_bus.rs (Proposed additions)
use std::path::Path;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

pub fn start_journal_logging(
    event_bus: &EventBus,
    workspace_root: &Path,
) -> tokio::task::JoinHandle<()> {
    let mut rx = event_bus.subscribe();
    let journal_path = workspace_root.join(".forge").join("journal.jsonl");

    tokio::spawn(async move {
        // Create directory structure if missing
        if let Some(parent) = journal_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&journal_path)
            .await
        {
            while let Ok(event) = rx.recv().await {
                if let Ok(serialized) = serde_json::to_string(&event) {
                    let mut line = serialized;
                    line.push('\n');
                    let _ = file.write_all(line.as_bytes()).await;
                }
            }
        }
    })
}
```

---

### 2. CLI Command: `anvil history`
The `anvil history` command allows users to inspect a log of past sandbox operations in the active workspace.

#### Sorting, Formats, and Schema
- **CLI Options**:
  - `--sort <newest|oldest>`: Sorts operations based on the start timestamp (Default: `newest`).
  - `--format <table|json|jsonl>`: Output layout (Default: `table`).
  - `--limit <number>`: Caps the number of displayed operations.
- **Aggregating Events into Operations**:
  - Scan `.anvil/journal.jsonl` line-by-line.
  - Parse each line as an `Event` and group by `operation_id`.
  - For each `operation_id` group:
    - **Start Time**: The timestamp of the first event.
    - **Runtimes**: Collected unique set of runtimes affected.
    - **Primary Phase/Action**: The first non-generic phase or operation name.
    - **Duration**: Difference between the last event's timestamp and the first event's timestamp.
    - **Status**: The status of the terminal event (e.g. `Success` if the final event status is `Finished`, `Failure` if any event is `Failed(msg)`).

#### Output Layout Examples
**Table Format**:
```
Operation ID      Start Time           Runtimes  Primary Phase  Status    Duration
----------------------------------------------------------------------------------
op-1719875896000  2026-07-01 13:18:16  node      Sync           Success   1.2s
op-1719875850000  2026-07-01 13:17:30  python    Repair         Failure   0.8s
```

---

### 3. CLI Command: `anvil explain <runtime>`
The `anvil explain <runtime>` command provides detailed troubleshooting information explaining how a specific runtime's version was resolved, where it's cached, and its metadata.

#### Output Details
- **Config Source**:
  - Path of `anvil.toml` currently active.
  - The exact requirement expression configured (e.g., `node = "^20.10.0"`).
- **Version Resolution Rules**:
  - Host Platform/Architecture details (e.g., detected: `windows-x86_64`, normalized: `windows-x86_64`).
  - Version selection candidates list found in registry (e.g. `.anvil/metadata_cache.toml` or internal fallback).
  - Selected entry, resolved version, download URL, expected SHA-256 hash, and payload size.
  - Translation or emulation rules applied (e.g., Windows ARM64 fallback to x86_64).
- **Cache & Downloads**:
  - Cache storage location (e.g., `~/.anvil/runtimes/node/20.10.0/`).
  - Status of local archive download file (Present/Missing).
  - Status of extracted runtime directory (Present/Missing/Integrity verified).
  - Status of shims cache signature mapping.

---

### 4. CLI Command: `anvil trace <operation_id>`
The `anvil trace <operation_id>` command reads the operation journal and reconstructs a chronological, step-by-step trace of that operation.

#### CLI Output Design
- Read `.anvil/journal.jsonl` and filter events by the given `operation_id`.
- Sort events by timestamp.
- Print a styled tree representation of the execution flow.
- Highlight step status using color codes or console icons (e.g. `[+]` Started, `[~]` Progress, `[*]` Finished, `[-]` Failed).

**Example Output**:
```
Operation: op-1719875896000
Start Time: 2026-07-01 13:18:16
Runtimes: node

├── [13:18:16] [node] Phase: Download | Status: Started
│   └── Message: Downloading Node.js v20.10.0 from https://nodejs.org/...
├── [13:18:17] [node] Phase: Extract  | Status: Progress (50%)
│   └── Message: Extracting zip archive to staging area
├── [13:18:18] [node] Phase: Commit   | Status: Progress (90%)
│   └── Message: Promoting staging files to local runtime cache
└── [13:18:18] [node] Phase: Verify   | Status: Finished
    └── Message: Successfully installed Node.js v20.10.0 (Checksum matches)
```

---

### 5. CLI Command: `anvil events`
This command prints real-time events as they are broadcasted on the `EventBus`. Because multiple CLI commands run in separate OS processes, we must design how the event stream is shared.

#### Approaches Comparison

| Approach | Pros | Cons | Complexity |
|----------|------|------|------------|
| **Option A: IPC (Unix Sockets & Windows Named Pipes)** | True real-time stream; no disk write overhead; direct process-to-process communication. | High implementation overhead; requires setting up socket servers/clients; platform-specific Rust boilerplate. | High |
| **Option B: Journal File Tailing (`tail -f` equivalent)** | Extremely simple; zero setup; unified data pipeline; handles historical buffer naturally; platform independent. | Minor latency introduced by filesystem polling or watcher; depends on disk writes. | Low |

#### Recommendation
**Option B: Journal File Tailing** is highly recommended. It keeps the architecture clean and simple. The CLI command `anvil events` can just watch `.anvil/journal.jsonl` using a tail watcher (using standard file polling or the `notify` crate). If the user specifies `--live`, it starts at the end of the file and prints new line appends in real-time.

---

### 6. Stable Public API Interface Facade
To freeze the public interfaces for integrations (such as IDE extensions or automated scripts), we will design a stable API facade module under `crates/anvil-core/src/api/v1.rs`.

#### Module Architecture
- `crates/anvil-core/src/lib.rs` will export `pub mod api;`
- `crates/anvil-core/src/api/mod.rs` will export `pub mod v1;`
- `crates/anvil-core/src/api/v1.rs` contains the high-level struct `Engine` representing the sovereign sandbox engine runtime.

#### Proposed Rust API Definition
```rust
// crates/anvil-core/src/api/v1.rs
use std::path::{Path, PathBuf};
use crate::types::{LifecycleState, OperationResult, Event};
use crate::event_bus::EventBus;

pub struct Engine {
    workspace_root: PathBuf,
    cache_dir: PathBuf,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct RuntimeExplanation {
    pub name: String,
    pub version_req: String,
    pub resolved_version: String,
    pub platform: String,
    pub arch: String,
    pub download_url: String,
    pub expected_sha256: String,
    pub size_bytes: u64,
    pub is_cached: bool,
    pub extract_dir: PathBuf,
    pub emulation_applied: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct OperationSummary {
    pub operation_id: String,
    pub start_timestamp: String,
    pub runtimes: Vec<String>,
    pub primary_phase: String,
    pub duration_ms: u64,
    pub status: String,
}

impl Engine {
    /// Creates a new instance of the Engine facade.
    pub fn new(workspace_root: PathBuf, cache_dir: PathBuf) -> Self {
        Self { workspace_root, cache_dir }
    }

    /// Retrieves the current environment lifecycle status.
    pub fn get_status(&self) -> Result<LifecycleState, String> {
        // delegates to internal state logic
        unimplemented!()
    }

    /// Triggers resolution, locking, and synchronization of configured runtimes.
    pub async fn sync(&self, event_bus: &EventBus) -> Result<OperationResult, String> {
        // executes ResolveOperation, LockOperation, and SyncOperation in sequence
        unimplemented!()
    }

    /// Performs diagnosis and repairs broken runtimes.
    pub async fn repair(&self, event_bus: &EventBus) -> Result<OperationResult, String> {
        // executes RepairOperation
        unimplemented!()
    }

    /// Cleans local state, caches, and backups.
    pub async fn clean(&self) -> Result<OperationResult, String> {
        // executes CleanOperation
        unimplemented!()
    }

    /// Generates diagnostic information explaining runtime resolution.
    pub fn explain(&self, runtime: &str) -> Result<RuntimeExplanation, String> {
        // resolves configuration and maps cache layout
        unimplemented!()
    }

    /// Retrieves historical operation summaries.
    pub fn read_history(&self, limit: Option<usize>) -> Result<Vec<OperationSummary>, String> {
        // reads and groups .anvil/journal.jsonl lines
        unimplemented!()
    }

    /// Gets a chronological sequence of events for a single operation.
    pub fn trace_operation(&self, operation_id: &str) -> Result<Vec<Event>, String> {
        // filters journal lines by operation_id
        unimplemented!()
    }
}
```

---

### 7. Plan for 6 Architecture Decision Records (ADRs)
We will document core architecture choices via 6 ADR files under `docs/adr/` (or `openspec/adr/` depending on team storage conventions). Below is the proposed list and summary of each record:

#### ADR-0001: Operations Layer
- **Context**: The engine executes complex orchestration commands (up, sync, resolve, lock, clean, run) that require consistency and structure.
- **Decision**: Implement a command pattern trait `Operation` paired with an execution `Context` and a `Plan`. This separates resolution/planning from actual system modification, enabling dry-runs (`anvil plan`).
- **Status**: Accepted.

#### ADR-0002: Lifecycle Management
- **Context**: The engine must keep track of local sandbox state.
- **Decision**: Define a formal `LifecycleState` state-machine. Persist the current computed state in `.anvil/state.json`. Verify transitions on every operation.
- **Status**: Accepted.

#### ADR-0003: Extensible Runtime Providers
- **Context**: Different runtimes (node, python, go, bun, rust) require distinct resolution mechanisms but must share a unified resolution path.
- **Decision**: Define a polymorphic `RuntimeProvider` trait that abstracts version lookup and maps version requirements (`semver`) to metadata hashes.
- **Status**: Accepted.

#### ADR-0004: Transactional Sandbox Operations & Rollbacks
- **Context**: Interrupted downloads or extractions leave the local runtime cache in a corrupt state.
- **Decision**: Implement all installers using a staging-and-promote transactional pattern. Perform downloads and extracts into `.anvil/staging/`. Promote via directory rename. Perform backup renames for existing items and rollback to backup in case of failures.
- **Status**: Accepted.

#### ADR-0005: Event Bus & Operation Journal
- **Context**: Users and integrations need real-time status updates and historical auditing of sandbox events.
- **Decision**: Use a broadcast event channel in memory during execution. Persist events concurrently to `.anvil/journal.jsonl` as JSON Lines. Allow CLI live watching by tailing the journal file.
- **Status**: Accepted.

#### ADR-0006: Shim Architecture & Path Hijacking
- **Context**: Sandboxed tools must execute seamlessly without modifying user files.
- **Decision**: Generate lightweight wrappers (shims) inside `.anvil/bin/` pointing to the shim binary. The shim uses a generated signature cache `.anvil/shims.cache` to locate the correct target binary version for the workspace, modifying the execution path (`PATH`) on the fly.
- **Status**: Accepted.

---

### Affected Areas
- `crates/anvil-core/src/event_bus.rs` — Incorporate journal persist hooks.
- `crates/anvil-core/src/api/v1.rs` — Create the new stable API facade interface.
- `crates/anvil-core/src/lib.rs` — Re-export the API facade modules.
- `crates/anvil-cli/src/main.rs` — Add subcommands for `history`, `explain`, `trace`, and `events`.
- `docs/adr/` — Create ADRs 0001 through 0006.

### Recommendation
1. Implement the journal capture system using an async logger task listening to the `EventBus` and writing directly to `.anvil/journal.jsonl`.
2. Standardize `anvil events` on tailing the `.anvil/journal.jsonl` file as it keeps implementation complexity low and provides a single unified codebase path.
3. Expose all command logic via the stable public facade `Engine` inside `crates/anvil-core/src/api/v1.rs` to allow robust CLI integration and third-party programmatic consumption.

### Risks
- **IO Bottlenecks**: High-frequency telemetry updates might cause heavy IO writes if disk throughput is low. Mitigate by using buffered writes.
- **Journal Bloat**: Frequent command executions (especially in CI/CD) could grow the `.anvil/journal.jsonl` size. Mitigate by implementing a size cap or simple retention policy (e.g. discard lines older than 30 days during clean/gc operations).
- **Process Concurrency**: Multiple anvil instances writing to the same journal file simultaneously could interleaving characters. Mitigate by locking the file or using platform-level atomic appends for small payloads.

### Ready for Proposal
**Yes**. The plan is technically aligned with the codebase's existing structures, and the proposed commands and public APIs provide comprehensive observability features without introducing unstable runtime dependencies.

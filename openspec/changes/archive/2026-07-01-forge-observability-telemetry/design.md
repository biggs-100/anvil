# Design: Anvil Observability & Telemetry

## Technical Approach
Implement a local telemetry architecture utilizing an asynchronous Event Bus subscriber writing to a line-delimited JSON (NDJSON) journal file. Expose all execution and introspection functions through a stable `Engine` API facade, consumed by both CLI commands and programmatic integrations.

## Architecture Decisions

| ID & Decision | Option / Alternatives | Tradeoff | Decision / Rationale |
|---|---|---|---|
| **ADR-0001**: API Facade | Programmatic wrapper vs Direct module calls | Direct calls expose internal structs. Wrapper isolates internal changes. | Implement `Engine` facade in `api::v1` as the sole CLI entry point. |
| **ADR-0002**: Journal Format | NDJSON (.jsonl) vs SQLite | SQLite requires C bindings or heavy dependencies. NDJSON is human-readable, append-only, and fast. | Store structured events as line-delimited JSON under `.anvil/journal.jsonl`. |
| **ADR-0003**: Thread-Safe Writer | Sync blocking locks vs Thread-safe background task | Lock contention slows main threads. Background task isolates filesystem I/O. | Spawn tokio async task in `EventBus` receiving events via broadcast channel. |
| **ADR-0004**: Live Tailing | OS File System Watcher vs File Polling | Platform-native watchers (e.g. `notify`) add complexity. Polling is highly portable. | Implement seek-to-end + sleep loop CLI reader for `--live`. |
| **ADR-0005**: ASCII Tree Representation | Flat list output vs Hierarchical ASCII tree | Flat list is hard to read. Tree shows nested phase execution clearly. | Format parent-child relationship of operations via standard ASCII tree markers. |
| **ADR-0006**: CLI Formatting | Plain string output vs Structured tabulating | Raw text is unstructured. Tabular formats align logs for diagnostic scannability. | Style `history` and `explain` using clear tabular layouts. |

## Data Flow
```
[EventBus] ──(broadcast tx)──> [Async Subscriber Rx]
                                       │
                              (tokio background task)
                                       ▼
                              [Buffered File Append] ──> [.anvil/journal.jsonl]
                                       ▲
[CLI/API Facade] ◄──(poll/read)────────┘
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/anvil-core/src/api/v1.rs` | Create | Exposes `Engine` struct, v1 types, and unified public methods. |
| `crates/anvil-core/src/event_bus.rs` | Modify | Add background tokio spawn writing enqueued events to journal. |
| `crates/anvil-core/src/lib.rs` | Modify | Re-export `api` module under `api::v1`. |
| `crates/anvil-cli/src/main.rs` | Modify | Add `history`, `explain`, `trace`, `events` commands calling `Engine`. |
| `docs/adr/0001-stable-engine-facade.md` | Create | ADR-0001: Expose clean public Engine facade API. |
| `docs/adr/0002-ndjson-operation-journal.md` | Create | ADR-0002: Persist local telemetry in NDJSON format. |
| `docs/adr/0003-thread-safe-journal-writer.md` | Create | ADR-0003: Thread-safe background writing to journal file. |
| `docs/adr/0004-introspection-live-cli.md` | Create | ADR-0004: Introspection and live diagnostics CLI commands. |
| `docs/adr/0005-ascii-tree-trace-reconstruction.md` | Create | ADR-0005: Formatted hierarchical ASCII tree structure. |
| `docs/adr/0006-file-polling-live-tailing.md` | Create | ADR-0006: Portable file polling watch logic. |

## Interfaces / Contracts

```rust
// crates/anvil-core/src/api/v1.rs
pub struct Engine {
    pub workspace_root: std::path::PathBuf,
    pub cache_dir: std::path::PathBuf,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RuntimeExplanation {
    pub runtime: String,
    pub state: String,
    pub diagnostics: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct OperationSummary {
    pub id: String,
    pub runtime: String,
    pub duration_ms: u64,
    pub status: String,
}

impl Engine {
    pub fn new(root: std::path::PathBuf) -> Result<Self, String>;
    pub async fn get_status(&self) -> Result<String, String>;
    pub async fn sync(&self) -> Result<(), String>;
    pub async fn repair(&self) -> Result<(), String>;
    pub async fn clean(&self) -> Result<(), String>;
    pub async fn explain(&self, runtime: &str) -> Result<RuntimeExplanation, String>;
    pub async fn read_history(&self) -> Result<Vec<OperationSummary>, String>;
    pub async fn trace_operation(&self, id: &str) -> Result<String, String>; // Returns ASCII tree
}
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | NDJSON Serialization | Serialize/deserialize `Event` records and assert formatting. |
| Unit | Concurrent Append | Spawn multiple async threads writing to journal, verify line counts. |
| Integration | CLI Commands | Execute CLI subcommands `history`, `explain`, `trace` with mock journal files, assert console outputs. |
| Compiler | Facade Stability | Verify compilation of consumer crates using v1 types. |

## Migration / Rollout
No data migration required. The Operation Journal activates on first execution. The rollback plan requires removing `.anvil/journal.jsonl` and reverting code changes in `crates/`.

## Open Questions
- [ ] Should `journal.jsonl` rotate when it exceeds a size limit (e.g. 50MB)?

# Design: Forge Environment Lifecycle

## Technical Approach
We introduce a structured operations layer and plan engine in `forge-core` to decouple environment actions from CLI command handlers. All mutations proceed through a pre-computed Plan executed atomically. A `tokio::sync::broadcast` event bus reports real-time progress. Runtimes are installed to an isolated staging folder (`.forge/staging/<op_id>`) and promoted atomically using file-system directory renames. Sibling lifecycle coupling is managed via computed state checking and a flat state file (`.forge/state.json`).

```
           [Cli Commands (13)]
                   │
                   ▼ (plan / execute)
       [Operations Layer (Trait)]
         ├── plan() ────► [Plan Engine (Sync/RepairPlan)]
         └── execute() ──► [Atomic Transaction] ──► [Event Bus]
```

## Architecture Decisions

### Decision: Staging-to-Commit Directory Promotion

| Option | Tradeoff | Decision |
|---|---|---|
| Direct Extraction | Simple, but leaves corrupted/partial files on crash or network loss. | Rejected |
| Isolated Staging + Atomic Rename | Safe from partial extraction. Requires same-drive partition rename and Windows locks retry logic. | **Chosen** |

### Decision: State Persistence Model

| Option | Tradeoff | Decision |
|---|---|---|
| In-memory only | Zero disk footprint, but loses state between CLI invocations. | Rejected |
| Computed + Local Cache File | Fast. Uses lockfile, config, and folder health checks, falling back to `.forge/state.json`. | **Chosen** |

## Data Flow & State Machine

```
              ┌───────── init ────────┐
              ▼                       │
UNINIT ──► INITIALIZED ──► RESOLVED ──┴──► LOCKED
                            ▲                │
                            └──── repair ────┼──► SYNCED ──► READY
                                             │               │
                                             ▼               ▼
                                           DIRTY ◄─────── ACTIVE (run/shell)
                                             │
                                             ▼
                                           BROKEN
```

1. **Staging**: `Sync`/`Repair` download and extract into `.forge/staging/<op_id>/<name>-<version>/`.
2. **Promotion**: Back up target `~/.forge/runtimes/<name>/<version>/extracted` to `.forge/backup/`, perform `std::fs::rename`, delete backup. Rollback on failure.
3. **Event Bus**: Async events broadcasted from operations. CLI subscribes to print progress.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/forge-core/src/types.rs` | Modify | Define `LifecycleState`, `OperationResult`, `ChangeRecord`, `Event`. |
| `crates/forge-core/src/event_bus.rs` | Create | Event bus using `tokio::sync::broadcast`. |
| `crates/forge-core/src/operations/mod.rs` | Create | Define `Operation` trait, plans (`SyncPlan`, `RepairPlan`), and 10 operations. |
| `crates/forge-cli/src/main.rs` | Modify | Map 13 commands to operation layer and subscribe to progress. |

## Interfaces / Contracts

```rust
// crates/forge-core/src/operations/mod.rs
pub trait Plan: std::any::Any + Send + Sync {
    fn to_json(&self) -> serde_json::Value;
}

pub trait Operation: Send + Sync {
    fn name(&self) -> &str;
    fn plan(&self, ctx: &Context) -> Result<Box<dyn Plan>, String>;
    fn execute(&self, ctx: &mut Context, plan: Box<dyn Plan>) -> Result<OperationResult, String>;
}

// crates/forge-core/src/types.rs
#[derive(Debug, Clone, Serialize)]
pub struct OperationResult {
    pub status: OperationStatus,
    pub duration_ms: u64,
    pub warnings: Vec<String>,
    pub changes: Vec<ChangeRecord>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum OperationStatus { Success, Failure, Warning, Skipped }

#[derive(Debug, Clone, Serialize)]
pub struct ChangeRecord {
    pub path: String,
    pub action: String, // "added" | "modified" | "deleted"
}

#[derive(Debug, Clone, Serialize)]
pub enum EventStatus {
    Started,
    Progress(u8),
    Finished,
    Failed(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub timestamp: String,
    pub operation_id: String,
    pub runtime: String,
    pub phase: String,
    pub status: EventStatus,
    pub message: Option<String>,
}
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | State Transitions | Test transition checks from `LOCKED` to `READY` to `ACTIVE`. |
| Integration | Atomic Rollback | Simulate rename/extract failure and verify backup restoration. |
| Integration | Idempotency | Run `SyncOperation` twice; assert second run outputs `Skipped`. |

## Migration / Rollout
No migration required. Local environment directories are backwards-compatible; run `forge clean` to discard any legacy un-versioned runtimes.

## Open Questions
- [ ] Should we use a registry file lock to coordinate parallel CLI executions from different terminals?

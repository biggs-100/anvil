# Design: Anvil Explain Everything

## Technical Approach

Refactor `Commands::Explain` from a flat `{ runtime: String }` variant into a subcommand enum `ExplainCommands` with 5 variants: `Runtime`, `Operation`, `Context`, `Config`, `Profile`. Each variant maps to existing Engine/ContextEngine methods — no new data sources backend needed. All output follows the existing `{:<20} | {:<50}` table format from `print_explain_table`. The bare `anvil explain <name>` continues to work via clap defaulting the first positional variant.

## Architecture Decisions

| Decision | Option | Tradeoff | Choice |
|----------|--------|----------|--------|
| CLI shape | (a) Subcommand enum, (b) Flat args with string dispatch | (a) Type-safe, automatic help text, no manual parsing; (b) Less code churn | **(a)** — matches existing patterns (`AiCommands`, `EnvCommands`) |
| Backward compat | (a) `Runtime` as first variant with `{ name }`, (b) Manual prefix match | (a) clap maps `anvil explain foo` to `Runtime { name: "foo" }` automatically; (b) Fragile and non-standard | **(a)** — clap's positional arg in first variant handles this |
| Context output | (a) Parse `ForgeContext` fields into table rows, (b) Dump raw JSON | (a) Consistent UX with other explain output; (b) Cheap but breaks pattern | **(a)** — extract provider names, masked values, limits |
| Config output | (a) Render `ResolvedEnvironment.vars` + `metadata[].source` as table, (b) Key=value lines | (a) Shows the *why* behind each value (source level, interpolation); (b) Hides ValueSource info | **(a)** — ValueSource is the valuable insight |

## Data Flow

```
CLI parse → Commands::Explain { command }
    │
    ├── Runtime { name }
    │   └─ Engine::explain(&name) → RuntimeExplanation → print_explain_table()
    │
    ├── Operation { id }
    │   ├─ Engine::history(None) → find by id
    │   ├─ Engine::trace(&id) → TraceTree string
    │   └─ print_operation_table(summary, trace)
    │
    ├── Context
    │   ├─ ContextEngine::new()
    │   ├─ .register(...) 6 providers (same as `anvil context`)
    │   ├─ .query(&options) → AnvilContext
    │   └─ print_context_table(context)
    │
    ├── Config
    │   ├─ Engine::env_resolve(None) → ResolvedEnvironment
    │   └─ print_config_table(vars, metadata)
    │
    └── Profile
        ├─ AnvilConfig.profile → profile list
        ├─ get_active_profile() → name
        ├─ Engine::env_resolve(Some(&name)) → ResolvedEnvironment
        └─ print_profile_table(name, env_vars, precedence)
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/anvil-cli/src/main.rs` | Modify | Replace `Explain { runtime }` with `Explain { command: ExplainCommands }` + 5 subcommand variants + 4 new print helpers |

## Interfaces / Contracts

```rust
#[derive(Subcommand)]
enum ExplainCommands {
    /// Print runtime details (version, state, cache, shims)
    Runtime { name: String },
    /// Show operation details with event trace tree
    Operation { id: String },
    /// Show context providers, collected fields, and limits
    Context,
    /// Show resolved environment with source levels and secrets masking
    Config,
    /// Show active profile, its env overrides, and precedence chain
    Profile,
}
```

Handler functions (all in `main.rs`):

```rust
async fn explain_runtime(engine: &Engine, name: &str) -> Result<(), String>     // wraps existing
fn print_operation_table(summary: &OperationSummary, trace: &str)
fn print_context_table(ctx: &ForgeContext)
fn print_config_table(resolved: &ResolvedEnvironment)
fn print_profile_table(active_profile: &str, config: &ForgeConfig, resolved: &ResolvedEnvironment)
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Print helpers formatting | Snapshot table strings with known inputs |
| Integration | Backward compat (`anvil explain node`) | Run via clap test harness, compare output |
| Integration | Each subcommand dispatches correctly | `--help` shows 5 subcommands; each exits 0 |
| Integration | `anvil explain operation <unknown>` | Expects non-zero exit + error message |

## Migration / Rollout

No migration required. Binary change only — `Explain` struct serialization is not persisted. The flat `runtime: String` variant becomes a subcommand; all existing callers of `anvil explain <runtime>` continue to work unchanged.

## Open Questions

- [ ] Does `Engine::trace()` return data suitable for a summary table header (status, duration), or should we use `Engine::history()` + `Engine::trace()` together?
- [ ] For `print_config_table`, should secret values show `[MASKED]` or should we let `ResolvedEnvironment` handle masking downstream?

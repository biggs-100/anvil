# Design: forge-benchmark

## Technical Approach

Add `anvil benchmark` as a clap subcommand in anvil-cli. All 5 benchmarks call existing Engine/DiagnosticEngine/ContextEngine APIs wrapped in `std::time::Instant`. Benchmark logic lives in a new `crates/anvil-cli/src/benchmark.rs` module to keep `main.rs` clean. Results printed as a formatted table; `--json` outputs `serde_json::Value`.

## Architecture Decisions

| Option | Tradeoffs | Decision |
|--------|-----------|----------|
| `benchmark.rs` vs inline main.rs | Inline bloats main.rs (now 2056 lines) | New `benchmark.rs` module — follows existing `mod jsonrpc;` pattern |
| Module in anvil-cli vs anvil-core | anvil-core already exposes all needed APIs; no new core logic needed | anvil-cli — keeps core free of CLI concerns |
| `std::time::Instant` vs `std::time::SystemTime` | Instant is monotonic, SystemTime can jump | `std::time::Instant` for wall-clock duration |
| Fail-fast vs continue on error | Fail-fast loses the rest of the data | Continue — each benchmark result is independent; errors reported per-metric |

## Data Flow

```
CLI (anvil benchmark)
  │
  ├─ Benchmark 1: Engine::new() + sync()          [Instant start/stop]
  ├─ Benchmark 2: DiagnosticEngine::run(Fast)      [Instant start/stop]
  ├─ Benchmark 3: ContextEngine::query(6 providers) [Instant start/stop]
  ├─ Benchmark 4: Engine::new() + get_status()     [Instant start/stop]
  └─ Benchmark 5: DiagnosticEngine::run(Fast) → health_score [reuse B2, report score]

  │
  └─ Output: table (stdout) or JSON (--json)
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `crates/anvil-cli/src/benchmark.rs` | Create | Benchmark runner — 5 benchmark functions + table/JSON formatting |
| `crates/anvil-cli/src/main.rs` | Modify | Add `mod benchmark;`, `Commands::Benchmark` variant, and dispatch in `run_cli` |

## Interfaces / Contracts

```
// benchmark.rs — public API
pub struct BenchmarkResult {
    pub name: &'static str,
    pub duration_ms: Option<f64>,  // None = error
    pub health_score: Option<u8>,  // Only for health score benchmark
    pub error: Option<String>,
}

pub fn run_benchmarks(
    current_dir: &Path,
    json: bool,
    compare: bool,
) -> Result<(), String>;
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | `BenchmarkResult` formatting (table, JSON) | Test with mock durations — verify table alignment and JSON structure |
| Integration | All 5 benchmarks run without panic | `cargo test` in a temp anvil project dir |
| Integration | Error per-benchmark (bad project dir) | Run in a non-anvil directory, confirm remaining benchmarks continue |

## Migration / Rollout

No migration required. New command — no existing behavior changed.

## Open Questions

- None

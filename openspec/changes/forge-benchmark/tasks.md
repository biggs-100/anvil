# Tasks: forge-benchmark

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~160-190 |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | single-pr |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: Low

## Phase 1: Benchmark Module

- [x] 1.1 Create `crates/anvil-cli/src/benchmark.rs` — define `BenchmarkResult` struct with `name`, `duration_ms`, `health_score`, `error`
- [x] 1.2 Implement `benchmark_sync()` — wrap `Engine::sync()` in `Instant::now()`, return `BenchmarkResult`
- [x] 1.3 Implement `benchmark_diagnostics()` — wrap `DiagnosticEngine::run(Fast)` in `Instant`, return `BenchmarkResult`
- [x] 1.4 Implement `benchmark_context()` — wrap `ContextEngine::query(6 providers)` in `Instant`, return `BenchmarkResult`
- [x] 1.5 Implement `benchmark_launch()` — construct `Engine` + call `get_status()` wrapped in `Instant`, return `BenchmarkResult`
- [x] 1.6 Implement `benchmark_health()` — run `DiagnosticEngine::run(Fast)`, extract `health_score` from report, return `BenchmarkResult`
- [x] 1.7 Implement `run_benchmarks(current_dir, json, compare)` — run all 5 sequentially, continue on error, collect into `Vec<BenchmarkResult>`
- [x] 1.8 Implement table formatter — print aligned columns (metric, value, unit) with red highlight for health < 80
- [x] 1.9 Implement JSON formatter — serialize results as `serde_json::Value` with 5 metric keys

## Phase 2: CLI Integration

- [x] 2.1 Add `mod benchmark;` declaration to `crates/anvil-cli/src/main.rs`
- [x] 2.2 Add `Benchmark { json: bool, compare: bool }` variant to `Commands` enum
- [x] 2.3 Add dispatch arm `Commands::Benchmark { json, compare } => benchmark::run_benchmarks(...)` in `run_cli`
- [x] 2.4 Add `"benchmark"` to `BUILTIN_COMMANDS` list

## Phase 3: Testing

- [x] 3.1 Unit test: `BenchmarkResult` table formatting with mock durations — verify column alignment and unit display
- [x] 3.2 Unit test: `BenchmarkResult` JSON serialization — verify valid JSON with all 5 metric keys
- [x] 3.3 Unit test: error result renders `"error"` in output (table + JSON)
- [x] 3.4 Unit test: health score < 80 is highlighted in table output

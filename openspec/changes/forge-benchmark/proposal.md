# Proposal: forge-benchmark

## Intent

Measure real anvil engine performance in a deterministic, non-destructive way. Engineers need concrete numbers on sync, diagnostics, context extraction, and launch times to detect regressions and validate optimizations — currently all performance assessment is anecdotal.

## Scope

### In Scope
- 5 real benchmarks: sync time (`anvil up` path), diagnostic time (`anvil doctor --deep`), context extraction time, launch time (start + status), health score (DiagnosticEngine score)
- CLI command `anvil benchmark` with table output to stdout
- `--json` flag for machine-readable output
- Optional `--compare` flag to diff against last cached result

### Out of Scope
- Historical trend tracking, charts, dashboards
- Remote reporting or CI integration
- Benchmark warm-up or calibration passes
- Custom benchmark registration or plugin system

## Capabilities

### New Capabilities
- `forge-benchmark`: Performance benchmarking command — runs 5 real engine operations, measures wall-clock time, reports results to stdout

### Modified Capabilities
- None

## Approach

Add `anvil benchmark` subcommand via clap. Each benchmark runs the actual engine operation wrapped in `std::time::Instant` — no simulations. Results printed as a formatted table; `--json` switches to JSON. Optional `--compare` loads last result from a cache file in `.anvil/benchmark-cache.json`. Non-destructive: all operations are read-only (sync is measured via `anvil up` but does not commit destructive mutations — the resolve+lock+sync pipeline is already idempotent).

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/anvil-cli/src/commands/benchmark.rs` | New | Benchmark subcommand impl |
| `crates/anvil-cli/src/main.rs` | Modified | Register benchmark route |
| `crates/diagnostic-engine/src/lib.rs` | Read | Expose health score via public method |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Benchmark alters engine state | Low | All ops are read-only; sync is idempotent |
| Timings vary across machines | Medium | Document baseline requirements; `--compare` caches per-machine |

## Rollback Plan

Remove the benchmark subcommand registration and delete `benchmark.rs`. Cache file in `.anvil/` is isolated — no other code reads it.

## Dependencies

- None — uses existing Engine API surface only

## Success Criteria

- [ ] `anvil benchmark` prints 5 metrics with measured values (not zero/placeholder)
- [ ] `anvil benchmark --json` emits valid JSON
- [ ] `anvil benchmark --compare` works with cached prior run
- [ ] No state is modified (confirmed by `git status` and anvil state checks)

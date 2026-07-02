# Forge-Benchmark Specification

## Purpose

Measure real forge engine performance by running 5 actual engine operations and reporting wall-clock duration. Results are printed to stdout as a human-readable table or machine-readable JSON. All operations are read-only — benchmark MUST NOT mutate engine state.

## Requirements

### Requirement: Sync Time Benchmark

The system MUST measure wall-clock duration of `Engine::sync()` using `std::time::Instant`.

#### Scenario: Measure sync duration

- GIVEN a configured `Engine` instance with a valid project
- WHEN the sync time benchmark runs
- THEN the elapsed wall-clock time is recorded in milliseconds

#### Scenario: Sync fails gracefully

- GIVEN the engine project directory is unavailable
- WHEN the sync time benchmark runs
- THEN the error is captured and remaining benchmarks continue

### Requirement: Diagnostic Time Benchmark

The system MUST measure wall-clock duration of `DiagnosticEngine::run()` in Fast mode using `std::time::Instant`.

#### Scenario: Measure diagnostic duration

- GIVEN a `DiagnosticEngine` instance
- WHEN the diagnostic time benchmark runs
- THEN the elapsed wall-clock time is recorded in milliseconds

### Requirement: Context Time Benchmark

The system MUST measure wall-clock duration of `ContextEngine::query()` with 6 providers using `std::time::Instant`.

#### Scenario: Measure context extraction duration

- GIVEN a `ContextEngine` instance initialized with 6 providers
- WHEN the context time benchmark runs
- THEN the elapsed wall-clock time is recorded in milliseconds

### Requirement: Launch Time Benchmark

The system MUST measure wall-clock duration of constructing a new `Engine` and calling `get_status()`.

#### Scenario: Measure launch duration

- GIVEN no prior `Engine` instance exists
- WHEN the launch time benchmark constructs an `Engine` and calls `get_status()`
- THEN the elapsed wall-clock time is recorded in milliseconds

### Requirement: Health Score Benchmark

The system MUST run `DiagnosticEngine` in Fast mode and report the `health_score` from the resulting `DiagnosticReport`.

#### Scenario: Report health score

- GIVEN a `DiagnosticEngine` instance
- WHEN the health score benchmark runs
- THEN the `DiagnosticReport`'s `health_score` field is recorded
- AND a score below 80 SHOULD be highlighted in red in table output

### Requirement: Output Format

The system MUST print all 5 metrics. Default output MUST be a human-readable table with columns for metric and value, including units (ms or s). With `--json`, output MUST be a single valid JSON object.

#### Scenario: Default table output

- GIVEN benchmarks have completed
- WHEN results are printed to stdout
- THEN each metric appears with value and unit
- AND the table has aligned columns

#### Scenario: JSON output with --json flag

- GIVEN the `--json` flag is passed
- WHEN benchmarks complete
- THEN stdout contains a valid JSON object with 5 metric keys and numeric values

### Requirement: CLI Command

The system MUST register `forge benchmark` as a clap subcommand. It MUST run all 5 benchmarks sequentially. A single benchmark failure MUST NOT crash the entire command.

#### Scenario: Sequential execution

- GIVEN `forge benchmark` is invoked
- WHEN execution begins
- THEN each benchmark runs after the previous one completes
- AND all 5 metrics are reported

#### Scenario: Engine error during benchmark

- GIVEN an engine operation returns an error
- WHEN the failing benchmark completes
- THEN the error is logged to stderr
- AND remaining benchmarks continue
- AND the failing metric reports `"error"` instead of a value

#### Scenario: Completion under 30 seconds

- GIVEN all engine operations succeed
- WHEN `forge benchmark` runs to completion
- THEN the total elapsed time SHOULD be under 30 seconds

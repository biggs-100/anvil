# Plan Engine Specification

## Purpose

Define the Plan Engine interface which determines the differences between the current environment and target states, producing a deterministic, executable `SyncPlan` or `RepairPlan` before any mutation is applied.

## Requirements

### Requirement: Planning Before Mutation
The system MUST compute a declarative execution plan before performing any filesystem modification.
- The Planner MUST perform read-only checks on the host environment and config files to generate the plan.
- The plan MUST detail all proposed additions, updates, deletions, and shims to be created.
- The plan MUST be serializable to JSON for dry-run viewing and verification.

### Requirement: Sync vs Repair Plans
The system MUST produce plan variants corresponding to different operations:
- **SyncPlan**: Generated when bringing the environment from `LOCKED` to `READY` state. Contains a list of missing runtimes to download, extract, and paths to shim.
- **RepairPlan**: Generated when resolving `DIRTY` or `BROKEN` states. Contains missing shims, corrupted runtime binaries to re-download, and cache cleanup tasks.

#### Scenario: Sync Plan Generation
- GIVEN a lockfile requiring Python 3.11 (not local) and Node 18 (already local)
- WHEN a `SyncPlan` is computed
- THEN the plan MUST include a download task for Python 3.11, skip Node 18 download, and require shim cache regeneration.

#### Scenario: Repair Plan Generation
- GIVEN an active environment where the Python binary is missing from `.anvil/runtimes/python`
- WHEN a `RepairPlan` is computed
- THEN the plan MUST mark the Python runtime for complete re-installation and specify shim repair.

#### Scenario: Idempotent Plan Results in No-Op
- GIVEN a fully synchronized and verified `READY` environment
- WHEN a `SyncPlan` is computed
- THEN the plan MUST contain zero tasks and report `status` as up-to-date.

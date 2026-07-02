# Diagnostic Repair Planner Specification

## Purpose
Define the planner logic that converts a list of diagnostic `Finding` structs into a consolidated and structured `RepairPlan` composed of `QuickFixAction` steps.

## Requirements

### Requirement: QuickFixAction Mapping
The `RepairPlanner` MUST extract `QuickFix` definitions from a `DiagnosticReport` and map them to their corresponding `QuickFixAction` variants.

| Action Enum Variant | Target Finding Codes | Description |
|---|---|---|
| `WipeAndReextract` | FG005, FG006, FG007 | Deletes corrupted extraction folder and triggers fresh extraction |
| `RecreateShim` | FG011 | Re-creates missing or broken shim binary symlink/wrapper |
| `SetEnvVar` | FG009 | Prompt user or set the missing profile environment variable |
| `SetSecret` | FG008 | Directs the CLI to configure a credentials token |
| `RegenerateLockfile`| FG003, FG004 | Triggers fresh dependency lock resolution |
| `RegenerateShimsCache`| FG011 | Forces regeneration of compiled shim cache data |
| `AddToGitIgnore` | FG008 | Appends private paths/secrets files to `.gitignore` |

#### Scenario: Mapping Outdated Lockfile to Action
- GIVEN a finding with code `FG004` (Outdated Lockfile) containing `QuickFixAction::RegenerateLockfile`
- WHEN `RepairPlanner::plan` is called
- THEN the resulting `RepairPlan` MUST contain the `RegenerateLockfile` action

---

### Requirement: Plan Consolidation and Deduplication
The `RepairPlanner` MUST compile and consolidate actions to avoid duplicate executions.

#### Scenario: Deduplicating Multiple Runtime Re-Extractions
- GIVEN multiple findings with `QuickFixAction::WipeAndReextract` for `node v18.16.0` (e.g., from hash mismatch and execution failure)
- WHEN `RepairPlanner::plan` compiles the plan
- THEN the resulting plan MUST contain exactly one `WipeAndReextract` action for `node v18.16.0`

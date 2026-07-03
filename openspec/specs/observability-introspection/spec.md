# Observability Introspection Specification

## Purpose
Define CLI subcommands for sandboxed operation analysis: `history`, `explain`, `trace`, and `events`.

## Requirements

| Command | Requirement | Format Options |
|---|---|---|
| `anvil history` | MUST read and print past operations from the journal. | `--limit <N>`, `--format <table|json>`, sort by timestamp desc. |
| `anvil explain <runtime>` | MUST display resolved configurations, cache integrity status, and shim locations. | Standard output table/text. |
| `anvil trace <op_id>` | MUST filter journal events by operation UUID and print a hierarchical tree structure. | ASCII hierarchy tree. |
| `anvil events` | MUST stream journal events. With `--live`, MUST poll/watch and tail new entries. | Live tailing stdout. |

#### Scenario: History Limit and Format
- GIVEN a journal with 5 entries
- WHEN executing `anvil history --limit 2 --format json`
- THEN the CLI MUST output the 2 most recent operations in valid JSON format.

#### Scenario: Explain Bun Runtime Cache
- GIVEN a configured Bun runtime
- WHEN executing `anvil explain bun`
- THEN it MUST verify and print the cache status and registered shims.

#### Scenario: Hierarchical Trace
- GIVEN an operation with ID `uuid-123` containing nested sub-phases
- WHEN executing `anvil trace uuid-123`
- THEN the CLI MUST print the execution steps in an ASCII tree showing duration and hierarchy.

#### Scenario: Live Events Tailing
- GIVEN the command `anvil events --live` is running
- WHEN a new event is written to `.anvil/journal.jsonl`
- THEN the CLI MUST output the serialized event line to stdout immediately.

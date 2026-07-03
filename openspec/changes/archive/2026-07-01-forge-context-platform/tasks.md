Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: High

# Tasks: Anvil Context Platform

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 600-800 |
| 400-line budget risk | High |
| Chained PRs recommended | No |
| Suggested split | Single PR (size:exception) |
| Delivery strategy | ask-on-risk |
| Chain strategy | size-exception |

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | ACP Core Engine & Models | PR 1 | Base traits, engine, schema, handshake, unit tests |
| 2 | Context Providers & Security | PR 2 | Provider logic, masking, limits, unit tests |
| 3 | Exporters, Agent Adapters, CLI | PR 3 | Markdown/JSON/MCP exporters, Claude/Gemini/Aider adapters, CLI command, integration tests |

## Phase 1: ACP Core Engine & Models (PR 1)

- [x] 1.1 Create `crates/anvil-core/src/context/mod.rs` and define traits `ContextProvider`, `ContextExporter`, `AgentAdapter`.
- [x] 1.2 Implement the `ContextEngine` registry, capability negotiation handshake structs (JSON-RPC), and the `ForgeContext` schema struct.
- [x] 1.3 Update `crates/anvil-core/src/lib.rs` to re-export the `context` module and core structs/traits.
- [x] 1.4 Write unit tests in `crates/anvil-core/src/context/tests.rs` (implemented inline in mod.rs tests module) verifying JSON-RPC handshake logic and `ContextEngine` thread safety under concurrent queries.

## Phase 2: Context Providers & Security (PR 2)

- [x] 2.1 Implement `Runtime`, `Configuration`, `Diagnostics`, `Workspace`, `Environment`, and `Secrets` providers in `crates/anvil-core/src/context/mod.rs`.
- [x] 2.2 Implement strict value masking using `is_secret(key)` in Environment/Secrets providers and limit the Workspace directory crawler to a depth of 5 and max 1000 files.
- [x] 2.3 Write unit tests in `crates/anvil-core/src/context/tests.rs` (implemented inline in mod.rs tests module) for secret masking and depth/file limit enforcement on mock workspace structures.

## Phase 3: Exporters, Agent Adapters, and CLI (PR 3)

- [x] 3.1 Implement `JsonExporter`, `MarkdownExporter`, and `McpExporter` traits in `crates/anvil-core/src/context/mod.rs`.
- [x] 3.2 Implement `ClaudeCodeAdapter`, `GeminiCliAdapter`, `AiderAdapter`, and `ContinueAdapter` formatting.
- [x] 3.3 Add the `context` command to `Commands` enum in `crates/anvil-cli/src/main.rs`, parse `--format`, `--scope`, `--exclude`, and route execution to `ContextEngine`.
- [x] 3.4 Create CLI integration tests in `crates/anvil-cli/tests/context_cli_tests.rs` verifying dry-runs and output formats (json/markdown).

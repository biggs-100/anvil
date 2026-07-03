# Proposal: Anvil Context Platform

## Intent
Establish a sovereign context extraction engine and protocol (Forge Context Protocol - ACP) to aggregate project state (runtimes, config, diagnostics, workspace, environment, secrets metadata) into a unified, secure query interface for developer agents, CLI exporters, and MCP servers.

## Scope

### In Scope
- Define traits `ContextProvider`, `ContextExporter`, `AgentAdapter`, and struct `ContextEngine` in `crates/anvil-core/src/context/mod.rs`.
- Implement 6 concrete context providers (Runtime, Configuration, Diagnostics, Workspace, Environment, Secrets).
- Standardize a SemVer-versioned `ForgeContext` schema v1.
- Implement Json, Markdown, and McpExporter formats.
- Create adapters for Claude Code (XML), Gemini CLI, Aider, and Continue.
- Support scope filters (`--scope`) and exclusion filters (`--exclude-cache`, `--exclude-history`, `--exclude-git`, etc.).
- Define capability negotiation handshake payload.
- Enforce sovereign security boundary restricting plaintext secrets extraction.
- Expose via CLI command `anvil context [--format <json|markdown>] [--scope <scope>] [--exclude <exclusion>]`.

### Out of Scope
- Implementing external TCP/HTTP network listeners (use stdio-based pipelines/MCP).

## Capabilities

### New Capabilities
- `context-engine`: Central ACP state manager and aggregator.
- `context-providers`: Implementation of 6 context suppliers.
- `context-exporters`: JSON, Markdown, and MCP adaptors.
- `context-agent-adapters`: Claude, Gemini, and Aider custom formats.
- `context-cli-commands`: The CLI subcommand `context`.

### Modified Capabilities
- None

## Approach
Implement context aggregation in Rust under `crates/anvil-core/src/context/`. Standardize data structures with `serde` for serialization. Add the sub-command and parameter parser inside `crates/anvil-cli/src/main.rs`. Ensure zero plaintext secrets are processed using `mask_sensitive_text`.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/anvil-core/src/context/mod.rs` | New | Trait definitions, engine, providers, exporters, and adapters |
| `crates/anvil-core/src/lib.rs` | Modified | Re-export new context platform module |
| `crates/anvil-cli/src/main.rs` | Modified | Add `anvil context` CLI command parser and execution flow |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Secrets Leakage | Low | Check existence metadata only; filter and mask all string output fields |
| Token Inflation | Med | Enforce strict size/count limits on workspace directory scanning |

## Rollback Plan
Revert code changes in Git for `crates/anvil-cli/src/main.rs`, `crates/anvil-core/src/lib.rs`, and delete the `crates/anvil-core/src/context/` directory.

## Dependencies
- None

## Success Criteria
- [ ] Command `anvil context` executes successfully and outputs structured JSON or markdown.
- [ ] Secret keys presence is reported without showing plaintext values.
- [ ] Excluded paths are successfully pruned from file trees.
- [ ] Context files pass all unit and integration tests.

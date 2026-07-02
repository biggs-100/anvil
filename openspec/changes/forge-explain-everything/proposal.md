# Proposal: Forge Explain Everything

## Intent

`forge explain <runtime>` currently shows runtime version, state, cache and shim details. Users need the same visibility into operations, context extraction, resolved configuration, and profile overlays — all data already exists in the Engine/ContextEngine/resolver layers but has no CLI surface. This change extends explain to cover those four domains without adding new Engine API methods.

## Scope

### In Scope
- `forge explain operation <id>` — operation detail with events and trace tree
- `forge explain context` — provider summary, masked values, workspace limits
- `forge explain config` — resolved env vars with source levels and interpolation
- `forge explain profile` — active profile env, precedence chain
- Refactor `Commands::Explain` from flat `{runtime}` to subcommand enum preserving existing behavior
- Structured table output matching `print_explain_table` / `print_history_table` style

### Out of Scope
- New Engine API methods or data stores
- Explain for plugins (future)
- GUI/TUI explain panels
- Modifying existing `Engine::explain(runtime)` output format
- `--json` or `--format` flags (future, can be added later)

## Capabilities

### New Capabilities
None — all data sources exist in Engine, ContextEngine, resolver, and ForgeConfig.

### Modified Capabilities
- `cli-commands-lifecycle`: explain subcommands (runtime, operation, context, config, profile) added as new command entries with input/output contracts. Req table extended from 13 to 17 commands.

## Approach

1. Replace `Commands::Explain { runtime: String }` with `Commands::Explain { command: ExplainCommands }` subcommand enum.
2. `ExplainCommands::Runtime` — delegates to existing `Engine::explain()` + `print_explain_table`.
3. `ExplainCommands::Operation` — reads journal via `Engine::trace()`, formats as structured tree + summary table.
4. `ExplainCommands::Context` — calls `ContextEngine::query()`, prints which providers ran, masked keys, workspace limits.
5. `ExplainCommands::Config` — calls `Engine::env_resolve()`, prints each var with its `ValueSource` level and interpolated value.
6. `ExplainCommands::Profile` — reads `ForgeConfig.profile`, prints active profile name, its env vars, and the full precedence chain.
7. All output follows existing table format (`{:<20} | {:<50}` style).

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/forge-cli/src/main.rs` | Modified | Explain refactored to subcommands, 4 new print helpers |
| `crates/forge-core/src/api/v1.rs` | None | No new methods needed |
| `crates/forge-core/src/resolver.rs` | Read | Level/precedence data reused |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| None — read-only, no new state, backward-compatible CLI | Low | Explain subcommand preserves `forge explain <runtime>` as default |

## Rollback Plan

Revert `Commands::Explain` to flat `{runtime: String}`, remove subcommand enum and print helpers. No data migration needed.

## Dependencies

None — all data sources are in the same crate workspace.

## Success Criteria

- [ ] `forge explain operation <id>` shows op ID, runtime, duration, status, event phases
- [ ] `forge explain context` shows provider names and collected fields
- [ ] `forge explain config` shows each var with source level and interpolated value
- [ ] `forge explain profile` shows active profile and precedence chain
- [ ] `forge explain <runtime>` output unchanged

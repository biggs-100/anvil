# Tasks: Anvil Explain Everything

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 150–250 |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | single-pr |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: Low

## Phase 1: Refactor Explain Enum

- [x] 1.1 Changed `Commands::Explain { runtime: String }` to `Commands::Explain { args: Vec<String> }` for backward compat + manual dispatch via `ExplainSubcommand` enum
- [x] 1.2 Defined `enum ExplainSubcommand` with Runtime, Operation, Context, Config, Profile variants + parse logic
- [x] 1.3 Runtime is the default parser fallback — first unknown arg treated as runtime name
- [x] 1.4 Match arm in `run_cli()` delegates to `handle_explain()` → per-variant handlers

## Phase 2: Runtime and Operation Handlers

- [x] 2.1 `handle_explain` → `Runtime` variant calls `Engine::explain()` + `print_explain_table()`
- [x] 2.2 `explain_operation()` — `Engine::history(None)` find by ID + `Engine::trace(id)`
- [x] 2.3 `print_operation_table()` — renders op ID, runtime, duration (ms), status, event timeline

## Phase 3: Context Handler

- [x] 3.1 `explain_context()` — instantiates `ContextEngine`, registers 6 providers, calls `.query()`
- [x] 3.2 `print_context_table()` — renders provider status, workspace limits (files, depth), masked secrets

## Phase 4: Config and Profile Handlers

- [x] 4.1 `explain_config()` — calls `Engine::env_resolve(None)`, renders each var with `ValueSource` level
- [x] 4.2 `print_config_table()` — shows var name, source level, value; secrets marked `[MASKED]`
- [x] 4.3 `explain_profile()` — detects active profile via `get_active_profile()`, calls `Engine::env_resolve(Some(&name))`
- [x] 4.4 `print_profile_table()` — shows active profile name, profile vars with override status

## Phase 5: Testing

- [x] 5.1 Unit test `test_print_operation_table` — smoke test with known `OperationSummary` + trace
- [x] 5.2 Unit test `test_explain_parse_operation_missing_id` — parser returns error for missing operation ID
- [x] 5.3 Unit test `test_print_context_table` — smoke test with mock `ForgeContext` (json! values)
- [x] 5.4 Unit test `test_print_config_table` — smoke test with mock `ResolvedEnvironment` including secret masking
- [x] 5.5 Unit test `test_explain_parse_profile/test_config/test_context/test_operation` — parser validates all 5 subcommands + aliases
- [x] 5.6 Unit test `test_explain_parse_runtime_backward_compat` — `parse(["node"])` returns `Runtime { name: "node" }`
- [x] 5.7 Unit tests — each subcommand parse variant tested (operation, context, config, profile, empty)

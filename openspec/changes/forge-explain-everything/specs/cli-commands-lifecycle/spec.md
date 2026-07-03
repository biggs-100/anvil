# Delta for cli-commands-lifecycle

## MODIFIED Requirements

### Requirement: Unified CLI Command Input/Output Contracts

All CLI commands MUST interact with the lifecycle state machine and return consistent exit codes (0 for success, non-zero for failure) and formatted output.
(Previously: 13 commands; now 18 with explain subcommands)

| Command | Input | Output / Side-Effect | Target State |
|---|---|---|---|
| `init` | Path / Template | Creates `anvil.toml` | INITIALIZED |
| `resolve` | Config | Version manifest generated | RESOLVED |
| `lock` | Manifest | Writes `anvil.lock` | LOCKED |
| `sync` | Lockfile, `--force` | Downloads, verifies, extracts runtimes | READY |
| `up` | Config, Lockfile | Resolve → lock → sync | READY |
| `run` | Cmd + args | Runs command in env | ACTIVE |
| `shell` | Shell type | Launches interactive subshell | ACTIVE |
| `clean` | `--all` / Runtime | Deletes runtime folders | INITIALIZED |
| `gc` | `--dry-run` | Deletes orphaned cache | (Unchanged) |
| `status` | None | JSON state representation | (Unchanged) |
| `inspect` | Runtime name | Prints metadata, paths, env | (Unchanged) |
| `repair` | None | 5-step env repair pipeline | READY |
| `plan` | None | Prints SyncPlan / RepairPlan | (Unchanged) |
| `explain runtime <name>` | Runtime name | Prints version, cache, state | (Unchanged) |
| `explain operation <id>` | Operation ID | Prints summary, events, duration tree | (Unchanged) |
| `explain context` | None | Prints providers, masked values, limits | (Unchanged) |
| `explain config` | None | Prints resolved vars + source levels | (Unchanged) |
| `explain profile` | None | Prints active profile, vars, precedence | (Unchanged) |

#### Scenario: Running sync from LOCKED State

- GIVEN state is LOCKED
- WHEN `anvil sync` runs
- THEN system MUST download, verify, extract, promote, and set state to READY.

#### Scenario: Shell Activation

- GIVEN state is READY
- WHEN `anvil shell` runs
- THEN system MUST launch a subshell with shim paths prepended, state goes to ACTIVE.

#### Scenario: Run Command Execution

- GIVEN state is READY
- WHEN `anvil run python --version` runs
- THEN system MUST execute, output python version, exit 0.

#### Scenario: Explain Operation Details

- GIVEN operation `op-abc-123` exists
- WHEN `anvil explain operation op-abc-123` runs
- THEN output MUST show status, events from trace, and SHOULD show a nested duration tree.

#### Scenario: Explain Operation — Unknown ID

- GIVEN no operation with ID `op-xyz-999`
- WHEN `anvil explain operation op-xyz-999` runs
- THEN system MUST exit non-zero with an error message.

#### Scenario: Explain Context Providers

- GIVEN a workspace with `context.max_files = 100`
- WHEN `anvil explain context` runs
- THEN output MUST list each provider that ran and indicate masked values and workspace limits.

#### Scenario: Explain Context — No Providers

- GIVEN no context sources match any provider
- WHEN `anvil explain context` runs
- THEN output MUST list providers with empty results and exit 0.

#### Scenario: Explain Config Vars

- GIVEN env vars ANVIL_HOME, PATH, and a secret
- WHEN `anvil explain config` runs
- THEN output MUST show each var with source level, value, and `[MASKED]` for secrets.

#### Scenario: Explain Config — Empty

- GIVEN no env vars configured
- WHEN `anvil explain config` runs
- THEN output MUST indicate no vars found and exit 0.

#### Scenario: Explain Profile

- GIVEN `anvil.toml` has `dev` profile and `ANVIL_PROFILE=dev`
- WHEN `anvil explain profile` runs
- THEN output MUST show active profile, its vars, and which are overridden.

#### Scenario: Explain Profile — No Profile

- GIVEN no `ANVIL_PROFILE` and no default profile
- WHEN `anvil explain profile` runs
- THEN output MUST indicate no active profile and exit 0.

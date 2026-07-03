# Anvil Manifest Specification v1.0.0 (AMS)

**Version**: 1.0.0 | **Status**: Frozen

## Purpose

Defines file formats and resolution rules for reproducible dev environments: `anvil.toml`, `anvil.lock`, `anvil.env`, `anvil.local.toml`, `anvil.secrets`, plus resolution and interpolation.

## File Formats

### anvil.toml

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `workspace_id` | string | no | sha256(8 hex) of root | Unique ID |
| `runtimes` | table | no | `{}` | name→version constraint |
| `config.definitions.<key>` | table | no | — | Variable definition (type, required, default, pattern, description, secret) |
| `profile.<name>.env` | table | no | — | Profile env vars (TOML-typed, resolved to string) |

Types: `"string"`, `"integer"`, `"boolean"`.

### anvil.lock

TOML array of `[[runtime]]` entries:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Runtime name |
| `version` | string | yes | Resolved version |
| `platform` | string | yes | `"windows"`, `"macos"`, `"linux"` |
| `arch` | string | yes | `"x86_64"`, `"aarch64"` |
| `url` | string | yes | Download URL |
| `size` | u64 | yes | Bytes |
| `sha256` | string | yes | Hex checksum |
| `emulation` | table | no | `{requested, installed, reason}` |

### anvil.env

`KEY=VALUE` lines. Rules: `#` comments, empty lines skipped, first `=` splits key/value, whitespace trimmed, matching `"`/`'` stripped from values.

### anvil.local.toml

`[env]` block with TOML-typed values. NOT version-controlled.

### anvil.secrets

```toml
[secrets.DB_PASS]
provider = "keyring"  # or "file"
key = "anvil/prod/DB_PASS"
```

Supported providers: `"keyring"` (OS keyring, scoped by workspace_id), `"file"` (`.anvil/secrets.enc`).

## Resolution Stack

| Lvl | Source | Description |
|-----|--------|-------------|
| 1 | CLI overrides | `--var KEY=VAL` |
| 2 | System env | `ANVIL_VAR_<KEY>` (prefix stripped) |
| 3 | Local overrides | `anvil.local.toml [env]` |
| 2.5 | Plugin providers | ConfigurationProvider values |
| 4 | Secrets | `anvil.secrets` resolved |
| 5 | Env file | `anvil.env` parsed |
| 6 | Profile overlays | `[profile.<name>.env]` |
| 7 | Defaults/Manifest | `definitions.<key>.default` |

Higher levels override lower. Same level: last-resolved wins.

## Variable Interpolation

`${...}` placeholders resolved iteratively (max 10 passes).

| Expression | Resolves To |
|------------|-------------|
| `${workspace.root}` | Absolute path of workspace root |
| `${runtime.<name>.path}` | Installed runtime path |
| `${env.<KEY>}` | Resolved env var (falls back to process env) |
| `${<VAR>}` | Direct variable reference |

Circular references stop after 10 iterations (partial result returned).

## Validation

Config definitions validate resolved env: required missing (critical), type mismatch (critical), pattern mismatch (critical), invalid regex (warning). Critical failures block materialization.

## Requirements

### Requirement: Resolution SHALL follow precedence

**Scenario**: CLI beats env file
- GIVEN `anvil.env` has `KEY=env` and CLI passes `--var KEY=cli`
- WHEN resolved
- THEN `KEY` SHALL be `"cli"`

**Scenario**: Local beats profile
- GIVEN `anvil.local.toml` sets `KEY=local` and `profile.dev.env` sets `KEY=dev`
- WHEN resolved with profile `dev`
- THEN `KEY` SHALL be `"local"`

### Requirement: Lockfile SHALL carry integrity

**Scenario**: Emulation recorded
- GIVEN runtime resolved via emulation (arm64→x86_64)
- WHEN serialized
- THEN SHALL include `emulation.{requested, installed, reason}`

### Requirement: anvil.env SHALL parse correctly

**Scenario**: Quoted values stripped
- GIVEN `KEY="hello world"`
- WHEN parsed
- THEN value SHALL be `hello world`

**Scenario**: Comments ignored
- GIVEN `# KEY=val`
- WHEN parsed
- THEN line SHALL be skipped

### Requirement: Interpolation SHALL resolve placeholders

**Scenario**: Multiple patterns in one value
- GIVEN `PATH=${workspace.root}/bin:${env.PATH}`
- WHEN interpolated
- THEN both substitutions SHALL be resolved

**Scenario**: Unknown runtime errors
- GIVEN `${runtime.unknown.path}`
- WHEN interpolated
- THEN SHALL return error

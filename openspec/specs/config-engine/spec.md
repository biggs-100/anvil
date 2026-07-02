# Configuration Engine Specification

## Purpose

Define the unified 5-level configuration resolution stack, profile overlays, and variable interpolation engine for Forge.

## RFC-0012: Configuration Specification

### Manifest Extensions
The manifest `forge.toml` allows configuration definitions and schemas under the `[config]` section. A gitignored local configuration `forge.local.toml` enables local developer overrides.

```toml
# Example forge.toml
[config.definitions.DATABASE_URL]
type = "string"
required = true
pattern = "^postgres://.*"
description = "Database URL"

[config.definitions.MAX_CONN]
type = "integer"
default = 10
```

---

## Requirements

### Requirement: 5-Level Configuration Resolution

The configuration engine MUST resolve keys using the following priority order (highest to lowest):
1. Level 1: CLI Flags and System Env Overrides (`FORGE_VAR_<KEY>`)
2. Level 2: Local Developer Overrides (`forge.local.toml`)
3. Level 3: Secrets Providers (`forge.secrets`)
4. Level 4: Environment File (`forge.env`)
5. Level 5: Project Manifest (`forge.toml` defaults / schemas)

#### Scenario: Resolve Key Across Stack
- GIVEN `forge.toml` defines `DB_PORT=5432` and `forge.local.toml` defines `DB_PORT=6543`
- WHEN the configuration is resolved
- THEN the system MUST return `DB_PORT` as `6543`

---

### Requirement: Profile Overlays

The system MUST support active profile overlays (`development`, `production`, `ci`). Active profiles MUST override the default configuration blocks.

#### Scenario: Active Profile Application
- GIVEN `forge.toml` has a default `DB_HOST="localhost"` and `[profile.production.env]` has `DB_HOST="prod-db"`
- WHEN the configuration is resolved with the active profile set to `production`
- THEN the system MUST return `DB_HOST` as `prod-db`

---

### Requirement: Variables Interpolation

The system MUST interpolate variables in the format `${variable}` dynamically. It MUST support derived keys including `${workspace.root}`, `${runtime.<name>.path}`, and `${env.KEY}`.

#### Scenario: Derived Keys Interpolation
- GIVEN `workspace.root` is `/project` and a config value is `${workspace.root}/bin`
- WHEN the variable is interpolated
- THEN the system MUST return `/project/bin`

---

### Requirement: Plugin Configuration Providers in Resolution Stack

The Engine MUST accept `ConfigurationProvider` implementations registered via `PluginRegistry`. Plugin providers MUST form an additional resolution level between Level 2 (forge.local.toml) and Level 3 (forge.secrets), making the stack:
1. CLI Flags / `FORGE_VAR_<KEY>`
2. Local Developer Overrides (forge.local.toml)
3. **Plugin Configuration Providers** (new)
4. Secrets Providers (forge.secrets)
5. Environment File (forge.env)
6. Project Manifest (forge.toml)

(Previously: 5-level stack with no plugin extension point. Plugin providers slot between local overrides and secrets.)

#### Scenario: Plugin Provider Overrides Local Config
- GIVEN `forge.local.toml` defines `DB_PORT=6543` and a plugin ConfigurationProvider defines `DB_PORT=7890`
- WHEN the configuration is resolved
- THEN the system MUST return `DB_PORT` as `6543` (Level 2 beats Level 2.5)

#### Scenario: Plugin Provider Overridden by Secrets
- GIVEN a plugin ConfigurationProvider defines `API_KEY=plugin-key` and a secrets provider defines `API_KEY=secret-key`
- WHEN the configuration is resolved
- THEN the system MUST return `API_KEY` as `secret-key` (Level 3 beats Level 2.5)

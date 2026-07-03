# Delta for Configuration Engine

## ADDED Requirements

### Requirement: Plugin Configuration Providers in Resolution Stack

The Engine MUST accept `ConfigurationProvider` implementations registered via `PluginRegistry`. Plugin providers MUST form an additional resolution level between Level 2 (anvil.local.toml) and Level 3 (anvil.secrets), making the stack:
1. CLI Flags / `ANVIL_VAR_<KEY>`
2. Local Developer Overrides (anvil.local.toml)
3. **Plugin Configuration Providers** (new)
4. Secrets Providers (anvil.secrets)
5. Environment File (anvil.env)
6. Project Manifest (anvil.toml)

(Previously: 5-level stack with no plugin extension point. Plugin providers slot between local overrides and secrets.)

#### Scenario: Plugin Provider Overrides Local Config
- GIVEN `anvil.local.toml` defines `DB_PORT=6543` and a plugin ConfigurationProvider defines `DB_PORT=7890`
- WHEN the configuration is resolved
- THEN the system MUST return `DB_PORT` as `6543` (Level 2 beats Level 2.5)

#### Scenario: Plugin Provider Overridden by Secrets
- GIVEN a plugin ConfigurationProvider defines `API_KEY=plugin-key` and a secrets provider defines `API_KEY=secret-key`
- WHEN the configuration is resolved
- THEN the system MUST return `API_KEY` as `secret-key` (Level 3 beats Level 2.5)

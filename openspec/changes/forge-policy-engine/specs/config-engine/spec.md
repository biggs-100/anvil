# Delta for Configuration Engine

## ADDED Requirements

### Requirement: Policy Section in anvil.toml

The `anvil.toml` manifest MUST support an optional `[policy]` section for declarative pre-flight policy rules. The section is parsed by `PolicyEngine` and is independent of the existing `[config]` and `[profile]` sections.

| Rule | Type | Default |
|------|------|---------|
| `allow_network` | bool | `true` |
| `require_hashes` | bool | `false` |
| `forbid_unlocked` | bool | `false` |
| `minimum_health` | u8 | `0` |
| `required_profiles` | string[] | `[]` |
| `forbid_runtimes` | string[] | `[]` |

#### Scenario: Policy Section in Manifest

- GIVEN `anvil.toml` contains `[policy]` with `allow_network = false`
- WHEN the manifest is parsed
- THEN the `[policy]` block MUST be present in the parsed `ForgeConfig` with the specified value for `allow_network` and defaults for all other rules

#### Scenario: Policy Section Absent

- GIVEN `anvil.toml` does not contain a `[policy]` section
- WHEN the manifest is parsed
- THEN the `policy` field in `ForgeConfig` MUST be a `PolicyConfig` with all default values (not `None`)

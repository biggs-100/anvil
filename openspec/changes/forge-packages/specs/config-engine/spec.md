# Delta for config-engine

## ADDED Requirements

### Requirement: Packages Section Parsing

The configuration engine MUST parse a `[packages]` section in `anvil.toml` that contains a `pip` field of type `Option<String>`. The field specifies a relative or absolute path to a pip-compatible requirements file. This section is additive to the existing 6-level resolution stack — it does not participate in resolution but is consumed directly by the package installer.

#### Scenario: Packages Pip Parsed

- GIVEN `anvil.toml` contains `[packages]\npip = "requirements.txt"` and no `[packages]` section existed in prior versions
- WHEN the configuration is loaded into the typed config model
- THEN `config.packages.pip` MUST be `Some("requirements.txt")`

#### Scenario: Packages Section Absent

- GIVEN `anvil.toml` does NOT contain a `[packages]` section
- WHEN the configuration is loaded
- THEN `config.packages` MUST be `None`

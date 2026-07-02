# Plugin CLI Command Specification

## Purpose

Define the `CliCommand` trait for third-party CLI commands and the mechanism for registering, loading at startup, and routing plugin commands through the Forge CLI.

## Requirements

| Requirement ID | Description | Strength |
|---|---|---|
| REQ-CLI-001 | Plugins MUST implement `CliCommand` with `name`, `description`, and `execute` to expose subcommands. | MUST |
| REQ-CLI-002 | The CLI startup MUST load registered CliCommand instances from PluginRegistry before arg parsing. | MUST |
| REQ-CLI-003 | Plugin commands MUST be dispatched via `forge <plugin-command>` with existing built-in precedence. | MUST |

### Requirement: CliCommand Trait

Plugins exposing CLI commands MUST implement `CliCommand` providing: `name() -> &str` (the subcommand name), `description() -> &str` (help text), and `execute(args: &[String]) -> Result<(), Error>` (the handler).

#### Scenario: Plugin Command Registration
- GIVEN a plugin implementing `CliCommand` with `name = "mycmd"`
- WHEN the plugin's `register` method is called
- THEN the command MUST be stored in the registry accessible at `forge mycmd --help`

### Requirement: CLI Startup Loading

The CLI entry point MUST query `PluginRegistry` for registered `CliCommand` instances before parsing user arguments, merging them into the command table.

#### Scenario: Plugin Command Dispatch
- GIVEN the CLI has built-in commands `init` and `sync`, and a registered plugin command `mycmd`
- WHEN a user runs `forge mycmd --flag value`
- THEN the CLI MUST route to the plugin's `execute` with `["--flag", "value"]` and not treat `mycmd` as unknown

### Requirement: Built-in Command Precedence

Built-in commands MUST take precedence over plugin commands with the same name. A plugin registering a duplicate name MUST be rejected with a conflict error.

#### Scenario: Name Conflict Rejection
- GIVEN a built-in command `init` and a plugin registering `CliCommand` with `name = "init"`
- WHEN the CLI merges plugin commands at startup
- THEN the plugin command MUST be rejected, and a warning MUST report the conflict

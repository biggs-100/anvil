# Context CLI Commands Specification

## Purpose
Define the CLI options, output streams, scopes, and exclusion flags for the `forge context` command interface.

## Requirements

### Requirement: Command Invocation and Options
The CLI MUST support the `forge context` subcommand with arguments to filter, exclude, and format outputs.

| Option | Values | Description | Default |
|---|---|---|---|
| `--format` | `json`, `markdown`, `mcp`, `claude`, `gemini`, `aider` | Output format adapter | `json` |
| `--scope` | Comma-separated list of: `runtimes`, `config`, `diagnostics`, `workspace`, `environment`, `secrets` | Limit query to specified providers | All |
| `--exclude-git` | Boolean | Exclude `.git` folder and git-ignored files | `true` |
| `--exclude-cache` | Boolean | Exclude local cache/temp files | `true` |
| `--exclude-history` | Boolean | Exclude shell/cmd history files | `true` |

#### Scenario: Subcommand Default Output
- GIVEN a workspace with default configurations
- WHEN the user runs `forge context`
- THEN the system MUST print the aggregated context to `stdout` in minified JSON format, utilizing all six providers

---

### Requirement: Scope Filtering
The CLI MUST parse comma-separated scope values and only execute the providers matching those scopes.

#### Scenario: Scope Restriction to Runtimes and Config
- GIVEN a user request `forge context --scope runtimes,config`
- WHEN the CLI processes scopes
- THEN the ContextEngine MUST only run the Runtime and Configuration providers, returning an empty/null diagnostics block

---

### Requirement: Exclusion Processing
The CLI MUST pass exclusion flags to the Workspace and Environment providers to skip matching files or sensitive directories.

#### Scenario: Exclude Cache Folder
- GIVEN a workspace with a `.forge/cache` directory
- WHEN the user executes `forge context --exclude-cache`
- THEN the Workspace provider MUST skip scanning files under `.forge/cache`

---

### Requirement: Separation of Streams
All formatted context data MUST be printed to `stdout`. All trace logs, diagnostics, progress updates, and error messages MUST be written to `stderr`.

#### Scenario: Error Redirection to Stderr
- GIVEN a runtime error during context aggregation
- WHEN the user runs `forge context`
- THEN the system MUST print the error logs to `stderr` and exit with a non-zero code

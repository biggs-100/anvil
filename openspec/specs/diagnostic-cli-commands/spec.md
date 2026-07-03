# Diagnostic CLI Commands Specification

## Purpose
Define the CLI parameters, layout formats, and security safeguards for the commands `anvil doctor` and `anvil ai doctor`.

## Requirements

### Requirement: CLI Commands and Flag Parameters
The CLI MUST expose commands for running diagnostics.

| Command | Option / Flag | Output Format | Description |
|---|---|---|---|
| `anvil doctor` | None | UTF-8 Table | Runs default diagnostics in Fast mode |
| `anvil doctor` | `--deep` | UTF-8 Table | Runs default diagnostics in Deep mode |
| `anvil doctor` | `--json` | JSON string | Serializes results into raw JSON |
| `anvil ai doctor` | None | JSON string | Performs Deep diagnostic run formatted for LLM consumption |

#### Scenario: Running Doctor with JSON output
- GIVEN a workspace with a missing lockfile
- WHEN the user executes `anvil doctor --json`
- THEN the command MUST output a JSON string matching the `DiagnosticReport` schema and exit with a non-zero status

---

### Requirement: Structured Console Output Format
When executing without `--json`, the CLI MUST render a structured table.

| Column | Content |
|---|---|
| **Code** | Unique identifier (e.g., `FG001`) |
| **Severity**| Finding severity (INFO, WARNING, ERROR, CRITICAL) |
| **Category**| Area of issue (e.g., `manifest`, `secrets`) |
| **Message** | Human-readable description of the problem |

#### Scenario: Human-Readable Table Render
- GIVEN a list of active findings
- WHEN `anvil doctor` completes execution
- THEN the terminal MUST print a formatted table listing each finding with its code, severity, category, and message

---

### Requirement: Enforced Credential Masking
The engine and CLI commands MUST mask all sensitive values (credentials, tokens, keys) in both table and JSON outputs using `[MASKED]`.

#### Scenario: Masking Plaintext Secret Finding
- GIVEN a secret check finding for `GITHUB_TOKEN` carrying a format violation
- WHEN the command outputs the finding details to stdout
- THEN the printed message and any serialized JSON properties MUST NOT contain the plaintext token and MUST display `[MASKED]` instead

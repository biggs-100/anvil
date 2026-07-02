# Context Providers Specification

## Purpose
Define the query behavior, target schema, and limits for the six core context providers, enforcing strict sovereign security rules to prevent secrets leakage.

## Requirements

### Requirement: Provider Implementation and Schemas
The system MUST implement the six distinct context providers listed below.

| Provider | Target Schema | Limits / Exclusions |
|---|---|---|
| **Runtime** | Active runtimes, paths, versions | Current environment only |
| **Configuration** | Loaded workspace settings | Excludes local override values |
| **Diagnostics** | Active check statuses, error lists | Last 50 diagnostic events |
| **Workspace** | Directory hierarchy, file stats | Max depth 5, max 1000 files |
| **Environment** | Selected system environment variables | Excludes known sensitive keys |
| **Secrets** | Key presence status metadata | **Strictly metadata only** |

#### Scenario: Runtime Provider Version Fetch
- GIVEN a Node.js runtime environment active at version `20.11.0`
- WHEN the Runtime provider executes
- THEN it MUST return the runtime name `Node.js` and version `20.11.0`

---

### Requirement: Sovereign Security Rule
No provider SHALL collect, store, or output plaintext secret values. Sensitive environment variables, keyring values, and configuration strings matching secret patterns (e.g., keys, tokens, passwords) MUST be masked as `[REDACTED]`.

#### Scenario: Secret Variable Metadata Query
- GIVEN a system environment variable `API_SECRET_KEY=supersecret123`
- WHEN the environment and secrets providers execute
- THEN they MUST report that `API_SECRET_KEY` is present but MUST return `[REDACTED]` instead of `supersecret123`

---

### Requirement: Workspace Limit Safeguards
The Workspace provider MUST enforce strict limits to prevent token inflation:
- Maximum recursion depth of 5 subdirectories.
- Maximum total files indexed of 1000.
- Exclude files matched by `.gitignore` and CLI exclusion flags.

#### Scenario: Workspace Directory Limit Truncation
- GIVEN a repository containing 1500 files and directories at depth 6
- WHEN the Workspace provider scans the repository
- THEN it MUST truncate the scan to 1000 files and ignore all files at depth 6

---

### Requirement: Plugin-Registered Context Providers

The `ContextEngine` MUST accept `ContextProvider` implementations registered via `PluginRegistry`. Plugin context providers MUST implement the same `ContextProvider` trait as built-in providers and MUST be queried alongside them when building the `ForgeContext`.

(Previously: Only six built-in context providers existed. Plugin providers extend the context with domain-specific information.)

#### Scenario: Plugin Provider Adds Custom Context
- GIVEN a plugin registers a `ContextProvider` that reports Docker container status
- WHEN `ContextEngine::build_context()` is called
- THEN the engine MUST query the plugin provider and include its output in the aggregated `ForgeContext`

#### Scenario: Plugin Provider Error Does Not Block Context
- GIVEN a plugin ContextProvider that panics or returns an error
- WHEN `ContextEngine::build_context()` is called
- THEN the engine MUST skip the failed provider and include an error indicator in the context, without blocking other providers

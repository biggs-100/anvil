# Plugin Registry Specification

## Purpose

Define the core plugin trait, registry lifecycle (scanning, loading, dependency resolution, initialization), API version gating, and error handling for the Anvil plugin system.

## Requirements

| Requirement ID | Description | Strength |
|---|---|---|
| REQ-PLUG-001 | All plugins MUST implement the `Plugin` trait providing name, version, api_version, depends_on, and register. | MUST |
| REQ-PLUG-002 | PluginRegistry MUST discover plugins from `~/.anvil/plugins/` and via `Engine::register_plugin()`. | MUST |
| REQ-PLUG-003 | PluginRegistry MUST reject plugins whose `api_version` does not match `ANVIL_PLUGIN_API_VERSION`. | MUST |
| REQ-PLUG-004 | PluginRegistry MUST resolve dependencies as a DAG with cycle detection before initialization. | MUST |
| REQ-PLUG-005 | PluginRegistry MUST call each plugin's `register` method after successful dependency resolution, in topological order. | MUST |

### Requirement: Plugin Trait Contract

All plugins MUST implement the `Plugin` trait with: `name() -> &str`, `version() -> &str`, `api_version() -> &str`, `depends_on() -> &[&str]`, and `register(registry: &mut PluginRegistry)`.

#### Scenario: Plugin Registration With Dependencies
- GIVEN a plugin `A` that depends on plugin `B`
- WHEN both plugins are registered and the DAG is resolved
- THEN plugin `B` MUST be initialized before plugin `A`, and both are accessible via the registry

### Requirement: API Version Gating

The registry MUST validate `api_version` against `ANVIL_PLUGIN_API_VERSION`. Mismatched plugins MUST be rejected with a descriptive error containing the plugin name, expected version, and actual version.

#### Scenario: API Version Mismatch Rejection
- GIVEN a plugin declaring `api_version = "2.0.0"` and `ANVIL_PLUGIN_API_VERSION = "1.0.0"`
- WHEN the registry attempts to load it
- THEN the registry MUST reject the plugin and return an error containing "API version mismatch"

### Requirement: DAG Cycle Detection

The registry MUST perform a depth-first traversal of `depends_on` to detect cycles. If a cycle is found, initialization MUST abort and return an error naming the plugins involved.

#### Scenario: Cyclic Dependency Aborts Loading
- GIVEN plugin `A` depends on `B`, plugin `B` depends on `C`, and plugin `C` depends on `A`
- WHEN the registry resolves dependencies
- THEN the registry MUST abort with a cycle-detected error mentioning all three plugins

### Requirement: Filesystem Scanning

The registry MUST scan `~/.anvil/plugins/` for `.so`/`.dll`/`.dylib` (or recognized plugin artifacts). Invalid entries MUST be skipped with a warning, not a hard failure.

#### Scenario: Plugin Directory Scan
- GIVEN `~/.anvil/plugins/` contains one valid plugin artifact and one unrecognized file
- WHEN the registry scans the directory
- THEN the valid plugin MUST be loaded, and the unrecognized file MUST be skipped with a warning

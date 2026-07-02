# Runtime Providers Specification

## Purpose

Define modular contracts for runtime Providers to abstract language-specific resolution, asset mapping, and installation checks.

## Requirements

| Requirement ID | Description | Strength |
|---|---|---|
| REQ-PROV-001 | Providers MUST abstract target support check, dynamic version resolution, and download asset mapping. | MUST |
| REQ-PROV-002 | Providers MUST run pre-installation verification to check if a valid version exists on the host. | MUST |

### Requirement: Provider Contract

#### Scenario: Version and Asset Mapping
- GIVEN a Bun provider configured for the current platform
- WHEN requested to resolve version "1.1.0"
- THEN the provider MUST return the exact version, target download URL, archive size, and SHA-256 hash.

### Requirement: Pre-installation Check

#### Scenario: Pre-installed Runtime Detected
- GIVEN a system Go installation of "1.22.0" matching version constraints
- WHEN verification is executed
- THEN the system MUST skip downloading and utilize the pre-installed host runtime.

---

### Requirement: Plugin-Registered Runtime Providers

The Engine MUST accept `RuntimeProvider` implementations registered via `PluginRegistry`. Plugin providers MUST use the same `RuntimeProvider` trait as built-in providers and MUST be queried alongside them during runtime resolution.

(Previously: Only built-in runtime providers were available. Plugin providers add an extension point without modifying the core resolution logic.)

#### Scenario: Plugin Provider Contributes a Runtime
- GIVEN a plugin registers a `RuntimeProvider` for "Deno" via `PluginRegistry`
- WHEN the Engine resolves runtimes during initialization
- THEN the Deno provider MUST be queried alongside built-in providers, and if the host satisfies Deno's requirements, the resolver MUST return valid Deno version metadata

#### Scenario: Plugin Provider Precedence Over Built-in
- GIVEN a built-in `NodeProvider` and a plugin that registers a different `NodeProvider` implementation for the same runtime name
- WHEN the Engine resolves the Node.js runtime
- THEN the built-in provider MUST take precedence, and the plugin provider MUST be ignored (registered later in the list)

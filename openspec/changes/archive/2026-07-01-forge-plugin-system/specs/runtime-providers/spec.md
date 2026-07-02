# Delta for Runtime Providers

## ADDED Requirements

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

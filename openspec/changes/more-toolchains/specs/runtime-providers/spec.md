# Delta for Runtime Providers

## MODIFIED Requirements

### Requirement: Plugin-Registered Runtime Providers

The Engine MUST accept `RuntimeProvider` implementations registered via `PluginRegistry`. Plugin providers MUST use the same `RuntimeProvider` trait as built-in providers and MUST be queried alongside them during runtime resolution. Built-in providers now include `NodeProvider`, `PythonProvider`, `BunProvider`, `GoProvider`, `RustProvider`, `LlvmProvider`, and `JdkProvider`.

(Previously: Only five built-in runtime providers existed. LlvmProvider and JdkProvider are now registered alongside the existing five.)

#### Scenario: Plugin Provider Contributes a Runtime
- GIVEN a plugin registers a `RuntimeProvider` for "Deno" via `PluginRegistry`
- WHEN the Engine resolves runtimes during initialization
- THEN the Deno provider MUST be queried alongside built-in providers (including `LlvmProvider` and `JdkProvider`), and if the host satisfies Deno's requirements, the resolver MUST return valid Deno version metadata

#### Scenario: Plugin Provider Precedence Over Built-in
- GIVEN a built-in `NodeProvider` and a plugin that registers a different `NodeProvider` implementation for the same runtime name
- WHEN the Engine resolves the Node.js runtime
- THEN the built-in provider MUST take precedence, and the plugin provider MUST be ignored (registered later in the list)

## ADDED Requirements

### Requirement: New Built-in Providers Registered

#### Scenario: LlvmProvider Registered at Startup
- GIVEN the Engine initializes the runtime resolver
- WHEN the resolver builds its provider list
- THEN `LlvmProvider` MUST appear in the list alongside existing providers

#### Scenario: JdkProvider Registered at Startup
- GIVEN the Engine initializes the runtime resolver
- WHEN the resolver builds its provider list
- THEN `JdkProvider` MUST appear in the list alongside existing providers

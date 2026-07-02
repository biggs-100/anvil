# Delta for Context Providers

## ADDED Requirements

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

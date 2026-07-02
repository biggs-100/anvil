# Delta for Context Exporters

## ADDED Requirements

### Requirement: Plugin-Registered Context Exporters

The `ContextEngine` MUST accept `ContextExporter` implementations registered via `PluginRegistry`. Plugin exporters MUST implement the same `ContextExporter` trait as built-in exporters and MUST be invokable through the same export interface.

(Previously: Only three built-in exporters (Json, Markdown, MCP). Plugin exporters let users serialize context to custom formats.)

#### Scenario: Plugin Exporter Generates Custom Format
- GIVEN a plugin registers a `ContextExporter` named "yaml" that serializes to YAML
- WHEN `ContextEngine::export("yaml")` is called
- THEN the engine MUST dispatch to the plugin exporter and return the YAML-serialized context

#### Scenario: Plugin Exporter Name Conflict
- GIVEN a built-in exporter named "json" and a plugin registering a `ContextExporter` also named "json"
- WHEN the plugin exporter is registered
- THEN the registry MUST reject the plugin exporter and emit a conflict warning; the built-in exporter retains precedence

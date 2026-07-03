# Context Exporters Specification

## Purpose
Define the formatting, serialization, and translation rules for JsonExporter, MarkdownExporter, and McpExporter.

## Requirements

### Requirement: JsonExporter Output Format
The `JsonExporter` MUST output the `AnvilContext` struct as a valid JSON payload matching the v1.0.0 schema. The exporter MUST support both minified format (default for programmatic pipes) and pretty-printed format (via flag).

#### Scenario: Programmatic Minified JSON Output
- GIVEN a valid aggregated `AnvilContext` struct
- WHEN `JsonExporter` format is executed with minification enabled
- THEN it MUST return a single-line, non-whitespace-padded JSON string

---

### Requirement: MarkdownExporter Structure
The `MarkdownExporter` MUST format the context into a clean, human-readable markdown document containing structured headers and tables for each provider's data.

| Component | Heading Level | Expected Element |
|---|---|---|
| Main Title | `# Anvil Context Summary` | Title |
| Runtimes | `## Runtimes` | List or Table |
| Config | `## Configuration` | Key-Value Table |
| Diagnostics | `## Diagnostics` | Status Block |
| Workspace | `## Workspace Tree` | Fenced Code block of tree |

#### Scenario: Markdown Summary Generation
- GIVEN a valid aggregated `AnvilContext` struct
- WHEN `MarkdownExporter` formats the data
- THEN the output string MUST start with `# Anvil Context Summary` and contain tables for configuration and runtimes

---

### Requirement: McpExporter Integration
The `McpExporter` MUST implement the Model Context Protocol (MCP) specification. It MUST expose the active context as an MCP resource located at `anvil://context/active`.

#### Scenario: MCP Resource Read request
- GIVEN an active MCP server connection
- WHEN a client requests a read on resource `anvil://context/active`
- THEN the McpExporter MUST return the serialized context wrapped in an MCP Resource content payload

---

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

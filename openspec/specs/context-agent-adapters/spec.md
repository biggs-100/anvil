# Context Agent Adapters Specification

## Purpose
Define adapter schemas and translation rules mapping aggregated `AnvilContext` outputs to formats optimized for Claude Code, Gemini JSON, and Aider repo map files.

## Requirements

### Requirement: Claude Code XML Adapter
The Claude Code adapter MUST map the `AnvilContext` output to an XML-structured document. It MUST wrap all provider data blocks in tag structures matching Claude Code context injection conventions.

| Source Object | Target XML Tag |
|---|---|
| Root context | `<anvil_context>` |
| Runtimes | `<runtimes>` |
| Configuration | `<configuration>` |
| Diagnostics | `<diagnostics>` |
| Workspace | `<workspace_files>` |

#### Scenario: Wrap Context in XML Tags
- GIVEN a `AnvilContext` containing a node runtime
- WHEN the Claude Code adapter maps the context
- THEN the output MUST be a string containing `<anvil_context><runtimes><runtime name="node"/></runtimes></anvil_context>`

---

### Requirement: Gemini JSON Adapter
The Gemini adapter MUST structure the context into a nested JSON format optimized for Gemini system instructions. It MUST wrap the data in a `systemInstructionContext` envelope and specify tool availability.

#### Scenario: Translate to Gemini System Context JSON
- GIVEN a `AnvilContext` payload
- WHEN the Gemini adapter processes the payload
- THEN it MUST return a JSON object with top-level key `systemInstructionContext` containing the structured metadata

---

### Requirement: Aider Repo Map Adapter
The Aider adapter MUST translate the workspace file listing and diagnostic check failures into an Aider-compatible repository map. It MUST highlight critical source code paths and functions while pruning less relevant files.

#### Scenario: Generate Aider Repo Map File
- GIVEN a workspace with files `crates/anvil-core/src/lib.rs` and `README.md`
- WHEN the Aider adapter formats the map
- THEN the output map MUST include class/function signatures of `crates/anvil-core/src/lib.rs` and omit `README.md`

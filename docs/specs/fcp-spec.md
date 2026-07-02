# Forge Context Protocol v1.0.0 (FCP)

**Version**: 1.0.0 | **Status**: Frozen

## Purpose

JSON-RPC 2.0 protocol for extracting structured dev environment context (runtimes, config, diagnostics, workspace, env, secrets). Enables AI agents, CLIs, and IDEs to query environment state without reverse-engineering.

## Transport

Any bidirectional transport (stdio, TCP). Messages are JSON-RPC 2.0 with `\n` delimiters.

## Protocol Methods

### fcp.handshake

| Field | Type | Description |
|-------|------|-------------|
| `jsonrpc` | string | MUST be `"2.0"` |
| `method` | string | MUST be `"fcp.handshake"` |
| `params.version` | string | Requested version |
| `params.capabilities.scopes` | string[] | Requested scopes |
| `params.capabilities.exporters` | string[] | Requested output formats |
| `id` | number/string | Match in response |

Scope aliases: `"runtimes"`→`"runtime"`, `"config"`→`"configuration"`. Response returns intersection of requested and supported capabilities. Unsupported scopes/exporters silently dropped.

### ForgeContext Schema

| Field | Type | Source |
|-------|------|--------|
| `schema_version` | string | `"1.0.0"` |
| `runtimes` | object | RuntimeProvider |
| `config` | object | ConfigurationProvider |
| `diagnostics` | object | DiagnosticsProvider |
| `workspace` | array | WorkspaceProvider |
| `environment` | object | EnvironmentProvider (masked) |
| `secrets_metadata` | object | SecretsProvider (masked) |

Provider timeout: 5000ms. Timeout → `{ "error": "Provider query timed out after 5000ms" }`. Panic → `{ "error": "Provider task panicked" }`.

## Provider Interface

`name()` + `collect(&ContextOptions) -> Result<Value, String>`.

| Provider | Scope | Output |
|----------|-------|--------|
| **Runtime** | `runtime` | Runtimes from forge.lock: name, version, platform, arch, url, size, sha256 |
| **Configuration** | `configuration` | forge.toml: workspace_id, runtimes, active_profile, definitions |
| **Diagnostics** | `diagnostics` | Health report: timestamp, mode, health_score, findings (max 50), elapsed_ms |
| **Workspace** | `workspace` | File tree: path, size, modified. Depth ≤5, files ≤1000. Binary/skip dirs excluded. Respects .gitignore. |
| **Environment** | `environment` | All env vars. Secret keys masked to `"[MASKED]"`. |
| **Secrets** | `secrets` | Secret metadata: key→{source, value: "[MASKED]"}. Checks keyring + env. |

## Exporter Interface

`name()` + `export(&ForgeContext) -> Result<String, String>`.

| Exporter | Name | Format |
|----------|------|--------|
| **JsonExporter** | `"json"` | Pretty or minified JSON |
| **MarkdownExporter** | `"markdown"` | Markdown tables per section |
| **McpExporter** | `"mcp"` | MCP envelope: `{contents: [{uri, mimeType, text}]}` |

Duplicate exporter names rejected (first registered wins).

## Agent Adapter Interface

`name()` + `adapt(&ForgeContext, &dyn ContextExporter) -> Result<String, String>`.

| Adapter | Name | Output |
|---------|------|--------|
| **ClaudeCode** | `"claude"` | XML `<forge_context>` with runtimes, config, diagnostics, workspace |
| **GeminiCli** | `"gemini"` | JSON with `systemInstructionContext` block |
| **Aider** | `"aider"` | Plain text repo map (Rust symbols extracted) |
| **Continue** | `"continue"` | JSON array: `{name, description, content}` |

## Security

`is_secret(key)` → true if key contains: secret, key, password, token, auth, credential, pass. Providers MUST replace values with `"[MASKED]"`. No plaintext secrets in ForgeContext.

## Requirements

### Requirement: Handshake SHALL negotiate capabilities

**Scenario**: Full capability match
- GIVEN client sends `fcp.handshake` with `"1.0.0"`, scopes `["workspace", "secrets"]`, exporters `["json"]`
- WHEN processed
- THEN response SHALL contain `version: "1.0.0"` and negotiated scopes/exporters as intersection

**Scenario**: Unknown scope dropped
- GIVEN client requests scope `"unknown_scope"`
- WHEN negotiated
- THEN `unknown_scope` SHALL NOT appear in response capabilities

### Requirement: Providers SHALL respect timeouts and limits

**Scenario**: Slow provider times out
- GIVEN a provider takes 6000ms
- WHEN query runs
- THEN after 5000ms its slot SHALL contain timeout error
- AND other providers SHALL return normally

**Scenario**: Workspace depth limit
- GIVEN a file at depth 6
- WHEN workspace provider collects
- THEN it SHALL NOT appear in output

### Requirement: Secrets SHALL be masked

**Scenario**: Environment masks sensitive keys
- GIVEN `API_KEY=secret123` and `DB_USER=forge`
- WHEN environment provider collects
- THEN `API_KEY` SHALL be `"[MASKED]"`, `DB_USER` SHALL be `"forge"`

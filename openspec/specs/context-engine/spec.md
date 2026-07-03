# Context Engine Specification (RFC-0014)

## Purpose
Define the Anvil Context Protocol (ACP) and the ContextEngine. It manages capability negotiation handshakes, concurrent provider execution, and outputs SemVer-versioned AnvilContext metadata.

## Requirements

### Requirement: ACP Handshake Negotiation
The engine MUST negotiate capabilities with clients using a JSON-RPC 2.0 handshake payload. The client and engine MUST exchange supported protocol versions, list of providers, and maximum payload size limits.

| Field | Type | Description |
|---|---|---|
| `protocol_version` | String | SemVer version (e.g., "1.0.0") |
| `providers` | Array<String> | Names of supported providers |
| `max_payload_bytes` | Integer | Maximum payload size limit |

#### Scenario: Handshake Version Match
- GIVEN a client request with `protocol_version` "1.0.0"
- WHEN the engine processes the handshake
- THEN it MUST return a response confirming "1.0.0" and the list of active providers

---

### Requirement: Concurrent Engine Execution
The ContextEngine MUST execute all active `ContextProvider` instances concurrently. It MUST enforce a strict timeout of 5000ms per provider and handle failures gracefully without failing the entire context request.

#### Scenario: Aggregation with Provider Timeout
- GIVEN a registered diagnostics provider that hangs for 6000ms
- WHEN the ContextEngine triggers execution
- THEN the engine MUST terminate that provider after 5000ms and return other aggregate data with a diagnostics error status

---

### Requirement: AnvilContext Metadata Schema v1.0.0
Aggregated context MUST conform to the SemVer-versioned AnvilContext schema v1.0.0.

| Field | Type | Description |
|---|---|---|
| `schema_version` | String | "1.0.0" |
| `runtimes` | Object | Active runtime states |
| `config` | Object | Project configuration values |
| `diagnostics` | Object | System health check results |
| `workspace` | Object | Directory tree and file metadata |
| `environment` | Object | System environment variables |
| `secrets_metadata` | Object | Presence flags for secrets |

#### Scenario: Schema Validation
- GIVEN a successfully aggregated context payload
- WHEN checked against the schema
- THEN the payload MUST contain `schema_version` "1.0.0" and all required top-level context objects

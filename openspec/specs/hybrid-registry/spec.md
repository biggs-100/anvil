# Hybrid Registry Specification

## Purpose

Coordinate runtime resolution using an offline-first metadata cache and a fallback remote registry.

## Requirements

| Requirement ID | Description | Strength |
|---|---|---|
| REQ-REG-001 | The registry MUST consult `.anvil/metadata_cache.toml` first for version and coordinate metadata. | MUST |
| REQ-REG-002 | The registry MUST fail resolution immediately if an exact uncached version is requested while offline. | MUST |
| REQ-REG-003 | The registry MUST resolve loose version ranges (e.g. "^20") to the latest matching cached version if offline. | MUST |
| REQ-REG-004 | The system MUST include a `RemoteRegistry` struct that fetches ARRS metadata from a URL. | MUST |
| REQ-REG-005 | `HybridRegistry` MUST chain resolution: local cache → remote ARRS registry → embedded defaults. | MUST |
| REQ-REG-006 | The system MUST cache remote responses locally in `.anvil/metadata_cache/`. | MUST |
| REQ-REG-007 | The system MUST fall back to cached data when the remote registry is unreachable. | MUST |
| REQ-REG-008 | The system MUST support a configurable registry URL (default: `https://registry.anvil.dev`). | MUST |
| REQ-REG-009 | The system SHOULD provide an offline mode that skips remote fetching entirely. | SHOULD |

### Requirement: Exact Version Offline Constraint

#### Scenario: Offline Exact Version Missing
- GIVEN the network is offline and Node "20.11.0" is not in `.anvil/metadata_cache.toml`
- WHEN resolving Node version "20.11.0"
- THEN the system MUST fail with a network/offline resolution error.

### Requirement: Range Resolution Offline Compatibility

#### Scenario: Offline Range Matching Cache
- GIVEN the network is offline and Node versions "20.10.0" and "20.9.0" are cached
- WHEN resolving Node version range "^20"
- THEN the system MUST resolve to "20.10.0" using cached metadata.

### Requirement: RemoteRegistry

A `RemoteRegistry` struct SHALL be introduced to perform HTTP GET requests against a configured base URL, requesting `{base_url}/{name}/{version}/metadata.toml`, parsing the response into ARRS metadata.

#### Scenario: Fetch Remote Registry on Cache Miss
- GIVEN the local cache has no entry for Python 3.13.0 and the remote registry is reachable
- WHEN resolving Python 3.13.0
- THEN the system MUST fetch from the remote registry and cache the result locally

### Requirement: Hybrid Resolution Chain

`HybridRegistry::resolve()` SHALL iterate: (1) local metadata cache, (2) remote ARRS registry, (3) embedded compiled-in defaults (via `default_with_internal()`).

#### Scenario: Serve from Cache on Network Failure
- GIVEN Python 3.13.0 was previously fetched and cached, and the network is now offline
- WHEN resolving Python 3.13.0
- THEN the system MUST return the cached metadata without attempting a remote fetch

#### Scenario: Fall Through to Embedded Defaults
- GIVEN no cache entry exists and the remote registry is unreachable
- WHEN resolving Python 3.13.0
- THEN the system MUST fall back to `default_with_internal()` embedded data

### Requirement: Registry URL Configuration

The registry URL SHALL be configurable via `anvil.toml` or environment variable, defaulting to `https://registry.anvil.dev`.

#### Scenario: Configure Custom Registry URL
- GIVEN `registry.url = "https://internal-mirror.corp/anvil"` in `anvil.toml`
- WHEN resolving any runtime
- THEN `RemoteRegistry` MUST use the configured URL as its base

### Requirement: Cache TTL and Refresh

Cached entries SHALL have a default TTL (e.g. 24 hours). Expired entries SHOULD trigger a background refresh; if refresh fails, stale data MAY be served.

#### Scenario: Stale Cache Served on Refresh Failure
- GIVEN a cached entry older than the TTL and the remote registry is unreachable
- WHEN resolving that runtime
- THEN the system MAY serve the stale cached entry and log a warning

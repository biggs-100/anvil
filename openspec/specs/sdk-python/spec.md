# SDK Python Specification

## Purpose

Defines the Python SDK package (`anvil-sdk`) that enables Python programs to control Anvil by spawning a `anvil --jsonrpc` subprocess and communicating via JSON-RPC 2.0 over stdio.

## Requirements

### Requirement: Package Structure

The SDK MUST be a Python package installable via `pip install anvil-sdk`. It MUST use only Python standard library — no external PyPI dependencies. It MUST be published on PyPI.

#### Scenario: pip install succeeds

- GIVEN a clean Python environment
- WHEN `pip install anvil-sdk` is run
- THEN installation MUST succeed
- AND `import anvil_sdk` MUST work without errors

### Requirement: Subprocess Lifecycle

The SDK MUST spawn `anvil --jsonrpc` as a subprocess using the `subprocess` module. It MUST terminate the subprocess on `client.close()` or when used as a context manager.

#### Scenario: Create Anvil client

- GIVEN `anvil` is installed and on `$PATH`
- WHEN `anvil_sdk.Anvil()` is called
- THEN it MUST spawn the anvil subprocess
- AND return a connected `Anvil` client
- AND when used as `with anvil_sdk.Anvil() as client:`
- THEN the subprocess MUST be terminated on exit from the `with` block

### Requirement: Method Surface

The `Anvil` class MUST provide Python-idiomatic methods mirroring the Rust SDK surface, all communicating via JSON-RPC:

- `status() -> dict`
- `sync() -> dict`
- `repair() -> dict`
- `clean() -> dict`
- `run(cmd, *args) -> dict`
- `context(fmt="json") -> dict`
- `explain(runtime) -> str`
- `history(limit=10) -> list`
- Environment: `env_list(), env_get(key), env_set(key, val), env_unset(key), env_resolve(key)`
- Secrets: `secret_set(key, val), secret_get(key), secret_list(), secret_remove(key)`

#### Scenario: Query context as dict

- GIVEN a connected `Anvil` client
- WHEN `client.context("json")` is called
- THEN it MUST return a Python `dict` representing the context data

#### Scenario: Handle connection error

- GIVEN a `Anvil` client whose subprocess has died
- WHEN any method is called
- THEN it MUST raise a `AnvilError` (or subclass of `Exception`)

### Requirement: Error Handling

All methods MUST raise `anvil_sdk.AnvilError` on failure. `AnvilError` MUST extend `Exception`.

### Requirement: Async Support

The SDK SHOULD provide async/await support via the `asyncio` module. All methods SHOULD have async variants prefixed with `async_` (e.g., `async_status()`).

#### Scenario: Async context query

- GIVEN a connected `Anvil` client in async mode
- WHEN `await client.async_context("json")` is called
- THEN it MUST return a `dict` with context data

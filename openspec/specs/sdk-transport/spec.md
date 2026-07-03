# SDK Transport Specification

## Purpose

Defines the JSON-RPC 2.0 transport protocol over stdin/stdout used by all Anvil SDKs. Anvil operates as a server process that reads JSON-RPC requests from stdin and writes responses to stdout, enabling language-agnostic programmatic access.

## Requirements

### Requirement: Transport Protocol

The system MUST implement JSON-RPC 2.0 over newline-delimited stdin/stdout. Each request MUST occupy exactly one line and each response MUST occupy exactly one line.

#### Scenario: Successful request-response roundtrip

- GIVEN anvil is running in `--jsonrpc` mode
- WHEN a valid JSON-RPC request `{"jsonrpc":"2.0","id":1,"method":"status","params":{}}\n` is written to stdin
- THEN anvil MUST write a JSON-RPC response to stdout with matching `id` and a `result` field
- AND the response MUST be followed by a newline

#### Scenario: Parse error returns error response

- GIVEN anvil is running in `--jsonrpc` mode
- WHEN malformed JSON is written to stdin (e.g., `not json\n`)
- THEN anvil MUST write `{"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}}\n` to stdout

### Requirement: Error Codes

The system MUST use these JSON-RPC error codes: `-32700` Parse error, `-32600` Invalid Request, `-32601` Method not found, `-32603` Internal error. Custom method errors MUST use codes `-32000` or above.

#### Scenario: Unknown method returns method-not-found

- GIVEN anvil is running in `--jsonrpc` mode
- WHEN a request with method `"nonexistent"` is written to stdin
- THEN anvil MUST respond with error code `-32601` and message `"Method not found"`

### Requirement: Request Format

Requests MUST use the format `{"jsonrpc":"2.0","id":<number>,"method":"<name>","params":{...}}`. The `id` field MUST be a non-negative integer. Notification requests (no `id`) MUST NOT produce a response.

#### Scenario: Notification request yields no response

- GIVEN anvil is running in `--jsonrpc` mode
- WHEN a request without an `id` field is written to stdin
- THEN anvil MUST NOT write any response to stdout

### Requirement: Concurrent Requests

The system MUST support concurrent (pipelined) requests. Responses MAY arrive out of order relative to their requests.

#### Scenario: Out-of-order responses for pipelined requests

- GIVEN anvil is running in `--jsonrpc` mode
- WHEN requests with `id: 1` (slow operation) and `id: 2` (fast operation) are written to stdin without waiting for responses
- THEN anvil MAY write the response for `id: 2` before the response for `id: 1`

### Requirement: Stream Flushing

The system MUST flush stdout after every JSON-RPC response to ensure the client receives it immediately.

### Requirement: Clean Shutdown on EOF

The system SHOULD exit cleanly with exit code 0 when stdin reaches EOF.

#### Scenario: EOF shuts down server

- GIVEN anvil is running in `--jsonrpc` mode
- WHEN stdin is closed (EOF)
- THEN anvil MUST exit with exit code 0 within 1 second

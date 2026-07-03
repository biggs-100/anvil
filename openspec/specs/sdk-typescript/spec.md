# SDK TypeScript Specification

## Purpose

Defines the TypeScript SDK package (`@anvil/sdk`) that enables Node.js programs to control Anvil by spawning a `anvil --jsonrpc` subprocess and communicating via JSON-RPC 2.0 over stdio.

## Requirements

### Requirement: Package Structure

The SDK MUST be an npm package named `@anvil/sdk`. It MUST provide TypeScript type definitions. It MUST use only Node.js standard library for subprocess management. It SHOULD work on Node.js 18+.

#### Scenario: npm install succeeds

- GIVEN a Node.js 18+ environment
- WHEN `npm install @anvil/sdk` is run
- THEN installation MUST succeed
- AND TypeScript compilation with `import { Anvil } from '@anvil/sdk'` MUST pass

### Requirement: Subprocess Lifecycle

The SDK MUST spawn `anvil --jsonrpc` using `child_process.spawn()`. It MUST kill the subprocess on client `.disconnect()` or when the process exits.

#### Scenario: Create Anvil client with types

- GIVEN `anvil` is installed and on `$PATH`
- WHEN `new Anvil()` is called
- THEN it MUST spawn `anvil --jsonrpc` as a child process
- AND return a typed `Anvil` instance
- AND `client.disconnect()` MUST kill the child process

### Requirement: Method Surface

The `Anvil` class MUST provide typed async methods mirroring the Rust SDK surface, all communicating via JSON-RPC:

- `status(): Promise<StatusInfo>`
- `sync(): Promise<SyncReport>`
- `repair(): Promise<RepairReport>`
- `clean(): Promise<CleanReport>`
- `run(cmd: string, ...args: string[]): Promise<RunOutput>`
- `context(fmt: ContextFormat): Promise<ContextData>`
- `explain(runtime: string): Promise<string>`
- `history(limit?: number): Promise<HistoryEntry[]>`
- Environment: `envList(), envGet(key), envSet(key, val), envUnset(key), envResolve(key)`
- Secrets: `secretSet(key, val), secretGet(key), secretList(), secretRemove(key)`

All types MUST be exported as interfaces/types.

#### Scenario: Run command with types

- GIVEN a connected `Anvil` client
- WHEN `await client.run("echo", ["hello"])` is called
- THEN it MUST return a `RunOutput` object with typed fields (`stdout: string`, `stderr: string`, `exit_code: number`)

#### Scenario: Handle process error

- GIVEN a `Anvil` client whose subprocess has exited
- WHEN any method is called
- THEN it MUST throw or return a rejected Promise with a `AnvilError`

### Requirement: Error Handling

Errors MUST extend `Error`. The `AnvilError` class MUST carry an optional `code` property for JSON-RPC error codes.

### Requirement: Type Definitions

All method parameter and return types MUST be defined as exported TypeScript interfaces. The package MUST ship `.d.ts` files.

#### Scenario: TypeScript compilation with types

- GIVEN a TypeScript project using `@anvil/sdk`
- WHEN `tsc --noEmit` is run
- THEN compilation MUST pass with all types resolved

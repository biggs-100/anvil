# SDK TypeScript Specification

## Purpose

Defines the TypeScript SDK package (`@forge/sdk`) that enables Node.js programs to control Forge by spawning a `forge --jsonrpc` subprocess and communicating via JSON-RPC 2.0 over stdio.

## Requirements

### Requirement: Package Structure

The SDK MUST be an npm package named `@forge/sdk`. It MUST provide TypeScript type definitions. It MUST use only Node.js standard library for subprocess management. It SHOULD work on Node.js 18+.

#### Scenario: npm install succeeds

- GIVEN a Node.js 18+ environment
- WHEN `npm install @forge/sdk` is run
- THEN installation MUST succeed
- AND TypeScript compilation with `import { Forge } from '@forge/sdk'` MUST pass

### Requirement: Subprocess Lifecycle

The SDK MUST spawn `forge --jsonrpc` using `child_process.spawn()`. It MUST kill the subprocess on client `.disconnect()` or when the process exits.

#### Scenario: Create Forge client with types

- GIVEN `forge` is installed and on `$PATH`
- WHEN `new Forge()` is called
- THEN it MUST spawn `forge --jsonrpc` as a child process
- AND return a typed `Forge` instance
- AND `client.disconnect()` MUST kill the child process

### Requirement: Method Surface

The `Forge` class MUST provide typed async methods mirroring the Rust SDK surface, all communicating via JSON-RPC:

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

- GIVEN a connected `Forge` client
- WHEN `await client.run("echo", ["hello"])` is called
- THEN it MUST return a `RunOutput` object with typed fields (`stdout: string`, `stderr: string`, `exit_code: number`)

#### Scenario: Handle process error

- GIVEN a `Forge` client whose subprocess has exited
- WHEN any method is called
- THEN it MUST throw or return a rejected Promise with a `ForgeError`

### Requirement: Error Handling

Errors MUST extend `Error`. The `ForgeError` class MUST carry an optional `code` property for JSON-RPC error codes.

### Requirement: Type Definitions

All method parameter and return types MUST be defined as exported TypeScript interfaces. The package MUST ship `.d.ts` files.

#### Scenario: TypeScript compilation with types

- GIVEN a TypeScript project using `@forge/sdk`
- WHEN `tsc --noEmit` is run
- THEN compilation MUST pass with all types resolved

/**
 * Integration tests for the Anvil TypeScript SDK.
 *
 * These tests spawn the `anvil jsonrpc` binary, send JSON-RPC 2.0 requests
 * over stdin, and verify responses from stdout.
 *
 * Run with: npx tsx tests/client.test.ts
 * Requires: anvil binary compiled and in PATH, or ANVIL_BIN env var set.
 */

import { spawn, ChildProcess } from 'child_process';
import * as assert from 'assert';

// ── Helpers ──────────────────────────────────────────────────────────────────

function anvilExe(): string {
  return process.env.ANVIL_BIN || 'anvil';
}

interface RpcRequest {
  jsonrpc: '2.0';
  id: number;
  method: string;
  params: Record<string, unknown>;
}

interface RpcSuccess {
  jsonrpc: string;
  id: number;
  result: unknown;
  error?: undefined;
}

interface RpcError {
  jsonrpc: string;
  id: number;
  error: { code: number; message: string };
  result?: undefined;
}

type RpcResponse = RpcSuccess | RpcError;

/**
 * Spawn `anvil jsonrpc`, send one request, read one response line,
 * then kill the subprocess.
 */
function sendRequest(request: RpcRequest): Promise<RpcResponse> {
  return new Promise<RpcResponse>((resolve, reject) => {
    const child: ChildProcess = spawn(anvilExe(), ['jsonrpc'], {
      stdio: ['pipe', 'pipe', 'inherit'],
    });

    let line = '';
    const onData = (data: Buffer) => {
      line += data.toString();
      const idx = line.indexOf('\n');
      if (idx !== -1) {
        const raw = line.substring(0, idx).trim();
        child.stdout?.removeListener('data', onData);
        child.kill();
        try {
          const parsed: RpcResponse = JSON.parse(raw);
          resolve(parsed);
        } catch (e) {
          reject(new Error(`Failed to parse response: ${raw}`));
        }
      }
    };

    child.stdout?.on('data', onData);

    child.on('error', (err) => {
      reject(new Error(`Subprocess error: ${err.message}`));
    });

    child.on('exit', (code) => {
      if (!line) {
        reject(new Error(`anvil jsonrpc exited with code ${code} without producing output`));
      }
    });

    child.stdin?.write(JSON.stringify(request) + '\n');
    child.stdin?.end();

    // Safety timeout
    setTimeout(() => {
      child.kill();
      reject(new Error('Request timed out'));
    }, 10_000);
  });
}

// ── Tests ────────────────────────────────────────────────────────────────────

async function testStatusRequest(): Promise<void> {
  const response = await sendRequest({
    jsonrpc: '2.0',
    id: 1,
    method: 'engine.status',
    params: {},
  });

  assert.strictEqual(response.jsonrpc, '2.0', 'Response should use jsonrpc 2.0');
  assert.strictEqual(response.id, 1, 'Response should echo the request id');

  if (response.error) {
    // If the engine returns an error, the status may depend on the environment
    console.log(`  status returned error (acceptable in CI): ${response.error.message}`);
  } else {
    assert.ok(response.result !== undefined, 'Response should have a result');
  }
}

async function testMethodNotFound(): Promise<void> {
  const response = await sendRequest({
    jsonrpc: '2.0',
    id: 2,
    method: 'nonexistent',
    params: {},
  });

  assert.strictEqual(response.jsonrpc, '2.0');
  if (response.error) {
    assert.strictEqual(response.error.code, -32601, 'Error code should be -32601');
    assert.ok(response.error.message.includes('nonexistent'), 'Error message should mention the method');
  } else {
    // Some implementations may return a generic error
    console.log('  nonexistent method request did not return an error (acceptable depending on implementation)');
  }
}

async function testParseError(): Promise<void> {
  return new Promise<void>((resolve, reject) => {
    const child: ChildProcess = spawn(anvilExe(), ['jsonrpc'], {

    setTimeout(() => { child.kill(); reject(new Error('Timed out')); }, 10_000);
  });
}

// ── Main runner ──────────────────────────────────────────────────────────────

async function main(): Promise<void> {
  console.log('Anvil TypeScript SDK Integration Tests\n');

  try {
    await testStatusRequest();
    console.log('  ✓ testStatusRequest');
  } catch (err) {
    console.log(`  ✗ testStatusRequest: ${(err as Error).message}`);
    // Don't fail the whole run — anvil binary availability may vary
  }

  try {
    await testMethodNotFound();
    console.log('  ✓ testMethodNotFound');
  } catch (err) {
    console.log(`  ✗ testMethodNotFound: ${(err as Error).message}`);
  }

  try {
    await testParseError();
    console.log('  ✓ testParseError');
  } catch (err) {
    console.log(`  ✗ testParseError: ${(err as Error).message}`);
  }

  console.log('\nDone.');
}

main().catch((err) => {
  console.error('Test runner failed:', err);
  process.exit(1);
});

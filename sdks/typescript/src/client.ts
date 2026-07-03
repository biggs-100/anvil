/** Anvil TypeScript SDK client — subprocess-based JSON-RPC client. */

import { spawn, ChildProcess } from 'child_process';
import {
  CleanReport,
  ContextData,
  ContextFormat,
  AnvilError,
  HistoryEntry,
  RepairReport,
  ResolvedEnvironment,
  RunOutput,
  RuntimeExplanation,
  StatusInfo,
  SyncReport,
} from './types';

// ── Internal JSON-RPC types ─────────────────────────────────────────────────

interface RpcRequest {
  jsonrpc: '2.0';
  id: number;
  method: string;
  params: Record<string, unknown>;
}

interface RpcResponse {
  jsonrpc: string;
  id: number;
  result?: unknown;
  error?: { code: number; message: string };
}

// ── Anvil client ────────────────────────────────────────────────────────────

/**
 * Client that controls a `anvil jsonrpc` subprocess.
 *
 * All methods communicate via JSON-RPC 2.0 over stdin/stdout.
 */
export class Anvil {
  private process: ChildProcess;
  private nextId = 0;
  private buffer: string;
  private resolveQueue: Array<{
    resolve: (value: RpcResponse) => void;
    reject: (reason: unknown) => void;
  }>;

  constructor(anvilPath = 'anvil') {
    this.buffer = '';
    this.resolveQueue = [];
    this.process = spawn(anvilPath, ['jsonrpc'], {
      stdio: ['pipe', 'pipe', 'inherit'],
    });

    this.process.stdout!.on('data', (data: Buffer) => {
      this.buffer += data.toString();
      this.processBuffer();
    });

    this.process.on('exit', (code) => {
      // Reject any pending requests
      for (const entry of this.resolveQueue) {
        entry.reject(new AnvilError(`anvil subprocess exited with code ${code}`));
      }
      this.resolveQueue = [];
    });

    this.process.on('error', (err) => {
      for (const entry of this.resolveQueue) {
        entry.reject(new AnvilError(`anvil subprocess error: ${err.message}`));
      }
      this.resolveQueue = [];
    });
  }

  // ── Lifecycle ──────────────────────────────────────────────────────

  /** Disconnect and kill the anvil subprocess. */
  disconnect(): void {
    if (this.process && !this.process.killed) {
      this.process.kill();
    }
  }

  // ── Core RPC ───────────────────────────────────────────────────────

  private async call<T>(method: string, params: Record<string, unknown> = {}): Promise<T> {
    if (!this.process || !this.process.stdout) {
      throw new AnvilError('anvil subprocess is not running');
    }

    this.nextId++;
    const request: RpcRequest = {
      jsonrpc: '2.0',
      id: this.nextId,
      method,
      params,
    };

    return new Promise<T>((resolve, reject) => {
      this.resolveQueue.push({ resolve: resolve as (value: RpcResponse) => void, reject });

      const line = JSON.stringify(request) + '\n';
      this.process.stdin!.write(line, (err) => {
        if (err) {
          // Remove from queue if write fails
          this.resolveQueue.pop();
          reject(new AnvilError(`failed to write request: ${err.message}`));
        }
      });
    }).then((response: unknown) => {
      const resp = response as RpcResponse;
      if (resp.error) {
        throw new AnvilError(resp.error.message, resp.error.code);
      }
      return resp.result as T;
    });
  }

  private processBuffer(): void {
    const lines = this.buffer.split('\n');
    // Keep the last partial line in the buffer
    this.buffer = lines.pop() || '';

    for (const line of lines) {
      if (!line.trim()) continue;

      try {
        const response: RpcResponse = JSON.parse(line);
        const entry = this.resolveQueue.shift();
        if (entry) {
          entry.resolve(response);
        }
      } catch {
        // Malformed response — skip
      }
    }
  }

  // ── Engine methods ─────────────────────────────────────────────────

  /** Get the current lifecycle state. */
  async status(): Promise<StatusInfo> {
    return this.call<StatusInfo>('engine.status');
  }

  /** Sync runtimes from lockfile. */
  async sync(): Promise<SyncReport> {
    return this.call<SyncReport>('engine.sync');
  }

  /** Repair corrupted or missing runtimes. */
  async repair(): Promise<RepairReport> {
    return this.call<RepairReport>('engine.repair');
  }

  /** Clean all local cache and state. */
  async clean(): Promise<CleanReport> {
    return this.call<CleanReport>('engine.clean');
  }

  /** Explain a runtime's configuration and cache status. */
  async explain(runtime: string): Promise<RuntimeExplanation> {
    return this.call<RuntimeExplanation>('engine.explain', { runtime });
  }

  /** Show past operations history. */
  async history(limit?: number): Promise<HistoryEntry[]> {
    return this.call<HistoryEntry[]>('engine.history', { limit: limit ?? 10 });
  }

  /** Execute a command inside the activated environment. */
  async run(cmd: string, ...args: string[]): Promise<RunOutput> {
    return this.call<RunOutput>('exec.run', { cmd, args });
  }

  /** Query contextual environment information. */
  async context(fmt: ContextFormat = 'json'): Promise<ContextData> {
    return this.call<ContextData>('context.get', { format: fmt });
  }

  // ── Environment methods ────────────────────────────────────────────

  /** List all environment variables. */
  async envList(): Promise<Record<string, string>> {
    return this.call<Record<string, string>>('env.list');
  }

  /** Get a single environment variable by key. */
  async envGet(key: string): Promise<string | null> {
    return this.call<string | null>('env.get', { key });
  }

  /** Set an environment variable. */
  async envSet(key: string, value: string): Promise<void> {
    return this.call<void>('env.set', { key, value });
  }

  /** Unset/remove an environment variable. */
  async envUnset(key: string): Promise<void> {
    return this.call<void>('env.unset', { key });
  }

  /** Resolve the unified environment. */
  async envResolve(key: string): Promise<ResolvedEnvironment> {
    return this.call<ResolvedEnvironment>('env.resolve', { key });
  }

  // ── Secrets methods ────────────────────────────────────────────────

  /** Set a secret. */
  async secretSet(key: string, value: string): Promise<void> {
    return this.call<void>('secret.set', { key, value });
  }

  /** Get a secret by key. */
  async secretGet(key: string): Promise<string | null> {
    return this.call<string | null>('secret.get', { key });
  }

  /** List all secret keys. */
  async secretList(): Promise<string[]> {
    return this.call<string[]>('secret.list');
  }

  /** Remove a secret. */
  async secretRemove(key: string): Promise<void> {
    return this.call<void>('secret.remove', { key });
  }
}

// Re-export types for convenience
export type { RunOutput } from './types';

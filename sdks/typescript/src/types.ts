/** Type definitions for @forge/sdk responses. */

export interface StatusInfo {
  state: string;
}

export interface SyncReport {
  status?: string;
}

export interface RepairReport {
  status?: string;
}

export interface CleanReport {
  status?: string;
}

export interface RuntimeExplanation {
  runtime: string;
  state: string;
  diagnostics: string[];
}

export interface OperationSummary {
  id: string;
  runtime: string;
  duration_ms: number;
  status: string;
}

export type HistoryEntry = OperationSummary;

export interface RunOutput {
  status: string;
  duration_ms: number;
  warnings: string[];
  changes: ChangeEntry[];
  diagnostics: string[];
}

export interface ChangeEntry {
  action: string;
  path: string;
}

export type ContextFormat = 'json' | 'json-pretty' | 'markdown' | 'mcp';

export interface ContextData {
  data: string;
}

export interface EnvVar {
  key: string;
  value: string;
}

export interface ResolvedEnvironment {
  vars: Record<string, string>;
}

export class ForgeError extends Error {
  public code?: number;

  constructor(message: string, code?: number) {
    super(message);
    this.name = 'ForgeError';
    this.code = code;
  }
}

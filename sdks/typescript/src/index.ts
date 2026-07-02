/**
 * @forge/sdk — TypeScript SDK for Forge environments.
 *
 * @example
 * ```typescript
 * import { Forge } from '@forge/sdk';
 *
 * const client = new Forge();
 * const status = await client.status();
 * console.log(status.state);
 * client.disconnect();
 * ```
 */

export { Forge } from './client';
export { ForgeError } from './types';
export type {
  StatusInfo,
  SyncReport,
  RepairReport,
  CleanReport,
  RuntimeExplanation,
  OperationSummary,
  HistoryEntry,
  RunOutput,
  ChangeEntry,
  ContextData,
  ContextFormat,
  EnvVar,
  ResolvedEnvironment,
} from './types';

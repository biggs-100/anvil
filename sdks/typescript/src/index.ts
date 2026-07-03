/**
 * @anvil/sdk — TypeScript SDK for Anvil environments.
 *
 * @example
 * ```typescript
 * import { Anvil } from '@anvil/sdk';
 *
 * const client = new Anvil();
 * const status = await client.status();
 * console.log(status.state);
 * client.disconnect();
 * ```
 */

export { Anvil } from './client';
export { AnvilError } from './types';
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

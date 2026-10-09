/**
 * Typed IPC with the Rust core (ADR 0007, ADR 0008).
 *
 * Features call the generated bindings in `./generated/<feature>.ts` from their
 * `api.ts`, wrap each call in {@link unwrap}, and read through TanStack Query
 * hooks whose `meta.tables` keeps them fresh via `DbChanged`.
 */
export { type IpcEvent, invalidateTables, listenToDbChanges, useIpcEvent } from './events';
export { IpcQueryProvider } from './ipc-query-provider';
export { createQueryClient, type IpcQueryMeta } from './query-client';
export { type CommandResult, IpcError, isAppError, unwrap } from './result';

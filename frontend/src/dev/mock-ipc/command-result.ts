/**
 * Command results for the mock modules. Kept apart from `mock-database.ts`,
 * which imports feature code: a mock that a feature reaches through its
 * generated bindings (for example `transcription`) would otherwise close an
 * import cycle and read the database class before it exists.
 */
import type { AppError } from '@/lib/ipc/generated/shared';
import type { CommandResult } from '@/lib/ipc/result';

/** A successful command result. */
export function ok<T>(data: T): CommandResult<T> {
  return { status: 'ok', data };
}

/** A failed command result. */
export function failure<T>(error: AppError): CommandResult<T> {
  return { status: 'error', error };
}

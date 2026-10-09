/**
 * Turns the `Result` values that generated commands resolve to into thrown,
 * typed errors, so TanStack Query sees an ordinary promise.
 *
 * Every error that reaches the UI from IPC is an {@link IpcError} with a
 * {@link ErrorCode}: commands return `AppError { code, message }`, and
 * failures Tauri raises itself (an ACL denial, a missing state) arrive as
 * plain strings, which become `internal` errors.
 */
import { type AppError, ERROR_CODES, type ErrorCode } from './generated/shared';

/** What a generated command resolves to (`ErrorHandlingMode::Result`). */
export type CommandResult<T> = { status: 'ok'; data: T } | { status: 'error'; error: AppError };

/** A failed IPC call. Branch on `code`; `message` is safe to show the user. */
export class IpcError extends Error {
  override readonly name = 'IpcError';

  constructor(
    readonly code: ErrorCode,
    message: string,
    options?: ErrorOptions,
  ) {
    super(message, options);
  }

  /** Wraps anything a command can reject with or return as its error. */
  static from(reason: unknown): IpcError {
    if (reason instanceof IpcError) return reason;
    if (isAppError(reason)) return new IpcError(reason.code, reason.message);
    if (reason instanceof Error) return new IpcError('internal', reason.message, { cause: reason });
    if (typeof reason === 'string') return new IpcError('internal', reason);
    return new IpcError('internal', 'The app could not complete the request.', { cause: reason });
  }
}

/** Resolves to the command's data, or throws its error as an {@link IpcError}. */
export async function unwrap<T>(pending: Promise<CommandResult<T>>): Promise<T> {
  let result: CommandResult<T>;
  try {
    result = await pending;
  } catch (thrown) {
    throw IpcError.from(thrown);
  }
  if (result.status === 'error') throw IpcError.from(result.error);
  return result.data;
}

/** True for a value shaped like the Rust `AppError`, validated at runtime. */
export function isAppError(value: unknown): value is AppError {
  if (typeof value !== 'object' || value === null) return false;
  if (!('code' in value) || !('message' in value)) return false;
  return ERROR_CODES.some((known) => known === value.code) && typeof value.message === 'string';
}

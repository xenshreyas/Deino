import { describe, expect, it } from 'vitest';

import { IpcError, unwrap } from './result';

describe('unwrap', () => {
  it('resolves to the data of an ok result', async () => {
    await expect(unwrap(Promise.resolve({ status: 'ok', data: 42 }))).resolves.toBe(42);
  });

  it('throws an AppError result as an IpcError with the same code and message', async () => {
    const result = {
      status: 'error',
      error: { code: 'not_found', message: 'meeting m was not found' },
    } as const;

    const error = await unwrap(Promise.resolve(result)).catch((thrown: unknown) => thrown);

    expect(error).toBeInstanceOf(IpcError);
    expect(error).toMatchObject({ code: 'not_found', message: 'meeting m was not found' });
  });

  it('turns failures Tauri raises itself into internal errors', async () => {
    const denied = Promise.reject(new Error('plugin:meetings|list not allowed'));

    const error = await unwrap(denied).catch((thrown: unknown) => thrown);

    expect(error).toMatchObject({ code: 'internal', message: 'plugin:meetings|list not allowed' });
  });
});

describe('IpcError.from', () => {
  it('keeps the code of anything shaped like an AppError', () => {
    expect(IpcError.from({ code: 'invalid', message: 'a note cannot be empty' }).code).toBe(
      'invalid',
    );
  });

  it('treats unknown codes and bare strings as internal', () => {
    expect(IpcError.from({ code: 'teapot', message: 'no' }).code).toBe('internal');
    expect(IpcError.from('state not managed').message).toBe('state not managed');
  });
});

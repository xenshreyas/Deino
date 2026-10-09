/**
 * Mock of `@/lib/ipc/generated/secrets`: which keys are "saved", in memory.
 * Like the real plugin it never hands a key back. The Claude key starts
 * saved, so the mock's Claude settings are complete.
 */
import type * as Generated from '@/lib/ipc/generated/secrets';

import { latency } from './latency';
import { failure, ok } from './mock-database';

/** The names of the keys "in the Keychain". */
const saved = new Set<Generated.SecretName>(['anthropic_api_key']);

export const commands = {
  async set(name, value) {
    await latency();
    if (value.trim() === '') return failure({ code: 'invalid', message: 'a key cannot be empty' });
    saved.add(name);
    return ok(null);
  },
  async status(name) {
    await latency();
    return ok(saved.has(name) ? 'stored' : 'missing');
  },
  async delete(name) {
    await latency();
    saved.delete(name);
    return ok(null);
  },
} satisfies typeof Generated.commands;

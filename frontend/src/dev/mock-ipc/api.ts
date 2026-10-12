/**
 * Mock of `@/lib/ipc/generated/api`: the local REST API, off on its default
 * port, with one read-only key already made. Turning it on "listens" at
 * once; port 80 is refused like the real settings refuse it.
 */
import type * as Generated from '@/lib/ipc/generated/api';

import { failure, ok } from './command-result';
import { latency } from './latency';

type ApiKey = Generated.ApiKey;
type ApiKeyId = Generated.ApiKeyId;
type ApiStatus = Generated.ApiStatus;

const NOW = '2026-10-12T09:00:00Z';

let settings: Generated.ApiSettings = { enabled: false, port: 47_810 };
let keys: ApiKey[] = [
  {
    id: '0199d6f0-0000-7000-8000-000000000001' as ApiKeyId,
    name: 'Raycast',
    scope: 'read',
    createdAt: '2026-10-01T10:00:00Z',
    lastUsedAt: '2026-10-11T18:30:00Z',
    revokedAt: null,
  },
];
let made = 1;

function status(): ApiStatus {
  return {
    ...settings,
    baseUrl: settings.enabled ? `http://127.0.0.1:${String(settings.port)}/v1` : null,
    error: null,
  };
}

function find(id: ApiKeyId): ApiKey | undefined {
  return keys.find((key) => key.id === id);
}

const notFound = { code: 'not_found', message: 'API key was not found' } as const;

export const commands = {
  async serverStatus() {
    await latency();
    return ok(status());
  },
  async configureServer(next: Generated.ApiSettings) {
    await latency();
    if (next.port < 1024) {
      return failure<ApiStatus>({ code: 'invalid', message: 'api.port must be 1024 or higher' });
    }
    settings = next;
    return ok(status());
  },
  async listKeys() {
    await latency();
    return ok([...keys]);
  },
  async createKey(key: Generated.NewApiKey) {
    await latency();
    made += 1;
    const id = `0199d6f0-0000-7000-8000-${String(made).padStart(12, '0')}` as ApiKeyId;
    const created: ApiKey = {
      id,
      name: key.name.trim(),
      scope: key.scope,
      createdAt: NOW,
      lastUsedAt: null,
      revokedAt: null,
    };
    keys = [created, ...keys];
    return ok({ key: created, token: `deino_${id.replaceAll('-', '')}_${'5e'.repeat(32)}` });
  },
  async revokeKey(id: ApiKeyId) {
    await latency();
    const key = find(id);
    if (key === undefined) return failure<ApiKey>(notFound);
    const revoked = { ...key, revokedAt: NOW };
    keys = keys.map((candidate) => (candidate.id === id ? revoked : candidate));
    return ok(revoked);
  },
  async deleteKey(id: ApiKeyId) {
    await latency();
    if (find(id) === undefined) return failure<null>(notFound);
    keys = keys.filter((key) => key.id !== id);
    return ok(null);
  },
} satisfies typeof Generated.commands;

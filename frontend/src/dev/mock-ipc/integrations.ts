/**
 * Mock of `@/lib/ipc/generated/integrations`: what a debug build lists. The
 * sandbox provider (any token connects it) and every planned provider as
 * WIP (D12), in Rust's order, plus two earlier deliveries for the log.
 */
import type * as Generated from '@/lib/ipc/generated/integrations';

import { failure, ok } from './command-result';
import { events } from './db';
import { fixtureUuid } from './fixture-ids';
import { latency } from './latency';

type IntegrationProvider = Generated.IntegrationProvider;
type Capability = Generated.Capability;

/** Mirrors `deino_integrations::catalog::planned_capabilities`. */
const PLANNED: readonly [IntegrationProvider, string, Capability[]][] = [
  ['slack', 'Slack', ['post_message']],
  ['notion', 'Notion', ['create_page']],
  ['linear', 'Linear', ['create_task']],
  ['todoist', 'Todoist', ['create_task']],
  ['asana', 'Asana', ['create_task']],
  ['jira', 'Jira', ['create_task']],
  ['monday', 'monday.com', ['create_task']],
  ['clickup', 'ClickUp', ['create_task']],
  ['trello', 'Trello', ['create_task']],
  ['hubspot', 'HubSpot', ['log_activity', 'create_task']],
  ['attio', 'Attio', ['log_activity', 'create_task']],
  ['salesforce', 'Salesforce', ['log_activity', 'create_task']],
  ['affinity', 'Affinity', ['log_activity']],
  ['zoho_crm', 'Zoho CRM', ['log_activity']],
  ['google_docs', 'Google Docs', ['create_page']],
  ['gmail', 'Gmail', ['read_context', 'post_message']],
  ['outlook_mail', 'Outlook', ['read_context', 'post_message']],
];

const SANDBOX_METHOD: Generated.ConnectMethod = {
  method: 'token',
  label: 'Sandbox token',
  usernameLabel: null,
  helpUrl: 'https://example.invalid/sandbox',
};

/** Change sequence numbers, apart from the other mocks' ranges. */
let seq = 2_000_000;

/** Tells the UI, as Rust's commit hook does, that integration rows changed. */
function changed(tables: Parameters<typeof events.dbChanged.emit>[0]['tables']) {
  seq += 1;
  void events.dbChanged.emit({ tables, seq });
}

/** Connected sandbox accounts. */
const connections: Generated.Connection[] = [];

/** Milliseconds per minute. */
const MINUTE_MS = 60_000;

/** `minutes` ago, as Rust serializes a `DateTime<Utc>`. */
function ago(minutes: number): string {
  return new Date(Date.now() - minutes * MINUTE_MS).toISOString().replace(/\.\d+Z$/, 'Z');
}

/** A delivery from an earlier session, `minutesAgo` minutes ago. */
function earlier(
  minutesAgo: number,
  key: string,
  outcome: Pick<Generated.Delivery, 'status' | 'external' | 'error' | 'attempts'>,
): Generated.Delivery {
  return {
    id: fixtureUuid(`delivery-${key}`) as Generated.DeliveryId,
    connectionId: null,
    provider: 'sandbox',
    capability: 'post_message',
    meetingId: null,
    idempotencyKey: `share:${key}`,
    ...outcome,
    createdAt: ago(minutesAgo),
    updatedAt: ago(minutesAgo),
  };
}

/** Newest first, like `integration_deliveries`. */
const deliveries: Generated.Delivery[] = [
  earlier(26, 'weekly-sync', {
    status: 'succeeded',
    external: { id: 'sandbox-message-2', url: null },
    error: null,
    attempts: 1,
  }),
  earlier(180, 'design-review', {
    status: 'failed',
    external: null,
    error: { code: 'unavailable', message: 'offline mode is on, so Sandbox cannot be reached' },
    attempts: 1,
  }),
];

function sandbox(): Generated.ProviderView {
  const capabilities: Capability[] = [
    'post_message',
    'create_page',
    'create_task',
    'log_activity',
    'read_context',
  ];
  return {
    id: 'sandbox',
    name: 'Sandbox',
    available: true,
    capabilities,
    methods: [SANDBOX_METHOD],
    app: null,
    connections: [...connections],
  };
}

function planned([id, name, capabilities]: (typeof PLANNED)[number]): Generated.ProviderView {
  return { id, name, available: false, capabilities, methods: [], app: null, connections: [] };
}

const UNSUPPORTED = failure<never>({
  code: 'invalid',
  message: 'Sandbox cannot be connected that way',
});

function newConnection(): Generated.Connection {
  const now = ago(0);
  return {
    id: fixtureUuid('sandbox-connection') as Generated.ConnectionId,
    provider: 'sandbox',
    accountId: 'sandbox',
    accountLabel: 'Sandbox workspace',
    authMethod: 'token',
    status: 'connected',
    scopes: [],
    tokenExpiresAt: null,
    connectedAt: now,
    updatedAt: now,
    lastUsedAt: null,
    lastError: null,
  };
}

function find(id: Generated.ConnectionId) {
  return connections.find((connection) => connection.id === id);
}

const notFound = (id: string) =>
  failure<never>({ code: 'not_found', message: `connection ${id} was not found` });

export const commands = {
  async providers() {
    await latency();
    return ok([sandbox(), ...PLANNED.map(planned)]);
  },
  async connect() {
    await latency();
    return UNSUPPORTED;
  },
  async cancelConnect() {
    await latency();
    return ok(false);
  },
  async connectToken(provider, input) {
    await latency();
    if (provider !== 'sandbox') return failure({ code: 'not_found', message: 'not available yet' });
    if (input.token.trim() === '')
      return failure({ code: 'invalid', message: 'paste the sandbox token' });
    const connection = find(newConnection().id) ?? newConnection();
    if (connections.length === 0) connections.push(connection);
    changed(['integration_connections']);
    return ok(connection);
  },
  async check(connection) {
    await latency();
    const found = find(connection);
    if (found === undefined) return notFound(connection);
    found.lastUsedAt = ago(0);
    changed(['integration_connections']);
    return ok(found);
  },
  async disconnect(connection) {
    await latency();
    const index = connections.findIndex(({ id }) => id === connection);
    if (index === -1) return notFound(connection);
    connections.splice(index, 1);
    changed(['integration_connections']);
    return ok(null);
  },
  async setApp() {
    await latency();
    return UNSUPPORTED;
  },
  async removeApp() {
    await latency();
    return ok(sandbox());
  },
  async targets() {
    await latency();
    return ok([{ id: 'general', name: '#general' }]);
  },
  async deliver(request) {
    await latency();
    const existing = deliveries.find(
      ({ idempotencyKey }) => idempotencyKey === request.idempotencyKey,
    );
    if (existing !== undefined) return ok(existing);
    const delivery = {
      ...earlier(0, request.idempotencyKey, {
        status: 'succeeded',
        external: { id: `sandbox-${String(deliveries.length + 1)}`, url: null },
        error: null,
        attempts: 1,
      }),
      idempotencyKey: request.idempotencyKey,
    };
    deliveries.unshift(delivery);
    changed(['integration_deliveries']);
    return ok(delivery);
  },
  async deliveries(limit) {
    await latency();
    return ok(deliveries.slice(0, Math.min(limit, 200)));
  },
} satisfies typeof Generated.commands;

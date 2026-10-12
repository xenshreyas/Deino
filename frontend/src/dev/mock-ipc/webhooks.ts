/**
 * Mock of `@/lib/ipc/generated/webhooks`: one Zapier endpoint with a short
 * delivery log, and one Make endpoint Deino turned off after a day of
 * failures, so the page shows its notice. Test sends to a URL containing
 * "fail" fail; every other test succeeds.
 */
import type * as Generated from '@/lib/ipc/generated/webhooks';

import { failure, ok } from './command-result';
import { events } from './db';
import { fixtureUuid } from './fixture-ids';
import { latency } from './latency';

type Webhook = Generated.Webhook;
type Delivery = Generated.Delivery;

/** Change sequence numbers, apart from the other mocks' ranges. */
let seq = 3_000_000;

/** Tells the UI, as Rust's commit hook does, that webhook rows changed. */
function changed() {
  seq += 1;
  void events.dbChanged.emit({ tables: ['webhooks', 'webhook_deliveries'], seq });
}

/** `minutes` ago, as Rust serializes a `DateTime<Utc>`. */
function ago(minutes: number): string {
  return new Date(Date.now() - minutes * 60_000).toISOString();
}

const FIELDS: Generated.PayloadFields = {
  notes: true,
  actionItems: true,
  transcript: false,
  insights: true,
  attendees: true,
  link: true,
  audioPath: false,
};

function endpoint(n: number, patch: Partial<Webhook>): Webhook {
  return {
    id: fixtureUuid(`webhook-${String(n)}`) as Generated.WebhookId,
    url: 'https://hooks.zapier.com/hooks/catch/1234/abcd/',
    label: '',
    events: ['meeting.ready'],
    fields: FIELDS,
    enabled: true,
    disabledReason: null,
    failureStreak: 0,
    failingSince: null,
    lastSuccessAt: ago(30),
    createdAt: ago(60 * 24 * 7),
    updatedAt: ago(30),
    ...patch,
  };
}

const webhooks: Webhook[] = [
  endpoint(1, { label: 'Zapier', events: ['meeting.ready', 'action_item.created'] }),
  endpoint(2, {
    label: 'Make',
    url: 'https://hook.eu1.make.com/fail-8f2k',
    events: ['notes.generated'],
    enabled: false,
    disabledReason: 'failing',
    failureStreak: 19,
    failingSince: ago(60 * 25),
    lastSuccessAt: null,
  }),
];

function delivery(webhook: Webhook, n: number, patch: Partial<Delivery>): Delivery {
  return {
    id: fixtureUuid(`delivery-${webhook.id}-${String(n)}`) as Generated.WebhookDeliveryId,
    webhookId: webhook.id,
    kind: 'meeting.ready',
    meetingId: 'meeting-fixture-standup' as Generated.MeetingId,
    status: 'succeeded',
    attempts: 1,
    nextAttemptAt: null,
    lastStatus: 200,
    lastError: null,
    createdAt: ago(30 + n * 90),
    finishedAt: ago(30 + n * 90),
    ...patch,
  };
}

const log = new Map<string, Delivery[]>([
  [
    webhooks[0]?.id ?? '',
    [
      delivery(endpoint(1, {}), 0, {}),
      delivery(endpoint(1, {}), 1, { kind: 'action_item.created', attempts: 3 }),
      delivery(endpoint(1, {}), 2, { kind: 'test' }),
    ],
  ],
]);

function find(id: Generated.WebhookId) {
  const webhook = webhooks.find((candidate) => candidate.id === id);
  return webhook === undefined
    ? failure<Webhook>({ code: 'not_found', message: `webhook ${id} was not found` })
    : ok(webhook);
}

const SECRET = 'whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSwYlE3uFJELVlN';

export const commands = {
  async list() {
    await latency();
    return ok([...webhooks]);
  },
  async create(input: Generated.WebhookInput) {
    await latency();
    if (!input.url.startsWith('https://') && !input.url.startsWith('http://localhost')) {
      return failure<Generated.CreatedWebhook>({
        code: 'invalid',
        message: 'URL must start with https:// (http:// only for localhost)',
      });
    }
    const webhook = endpoint(webhooks.length + 1, { ...input, lastSuccessAt: null });
    webhooks.push(webhook);
    changed();
    return ok({ webhook, secret: SECRET });
  },
  async update(id: Generated.WebhookId, input: Generated.WebhookInput) {
    await latency();
    const index = webhooks.findIndex((candidate) => candidate.id === id);
    const current = webhooks[index];
    if (current === undefined) return find(id);
    const cleared = input.enabled && !current.enabled;
    const next: Webhook = {
      ...current,
      ...input,
      ...(cleared ? { disabledReason: null, failureStreak: 0, failingSince: null } : {}),
    };
    webhooks[index] = next;
    changed();
    return ok(next);
  },
  async delete(id: Generated.WebhookId) {
    await latency();
    const index = webhooks.findIndex((candidate) => candidate.id === id);
    if (index < 0) return failure<null>({ code: 'not_found', message: 'not found' });
    webhooks.splice(index, 1);
    changed();
    return ok(null);
  },
  async rotateSecret(id: Generated.WebhookId) {
    await latency();
    const found = find(id);
    return found.status === 'ok' ? ok(`${SECRET}x`) : failure<string>(found.error);
  },
  async sendTest(id: Generated.WebhookId) {
    await latency();
    const found = find(id);
    if (found.status === 'error') return failure<Delivery>(found.error);
    const fails = found.data.url.includes('fail');
    const sent = delivery(found.data, (log.get(id)?.length ?? 0) + 10, {
      kind: 'test',
      status: fails ? 'failed' : 'succeeded',
      lastStatus: fails ? 500 : 200,
      lastError: fails ? 'the endpoint answered 500' : null,
      createdAt: ago(0),
      finishedAt: ago(0),
    });
    log.set(id, [sent, ...(log.get(id) ?? [])]);
    changed();
    return ok(sent);
  },
  async deliveries(id: Generated.WebhookId) {
    await latency();
    return ok(log.get(id) ?? []);
  },
} satisfies typeof Generated.commands;

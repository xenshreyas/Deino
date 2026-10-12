/**
 * Typed calls to the `webhooks` plugin. Each resolves to the data or throws
 * an `IpcError`; components use the hooks in `queries.ts`.
 */
import { commands } from '@/lib/ipc/generated/webhooks';
import { unwrap } from '@/lib/ipc/result';

import type { CreatedWebhook, Delivery, Webhook, WebhookId, WebhookInput } from './types';

export const webhooksApi = {
  /** Every endpoint, oldest first (never their secrets). */
  list: (): Promise<Webhook[]> => unwrap(commands.list()),
  /** Adds an endpoint; the answer carries its signing secret, this once. */
  create: (input: WebhookInput): Promise<CreatedWebhook> => unwrap(commands.create(input)),
  /** Changes an endpoint; turning it on again clears an auto-disable. */
  update: (id: WebhookId, input: WebhookInput): Promise<Webhook> =>
    unwrap(commands.update(id, input)),
  /** Removes an endpoint, its delivery log and its secret. */
  delete: (id: WebhookId): Promise<null> => unwrap(commands.delete(id)),
  /** A new signing secret, shown this once. */
  rotateSecret: (id: WebhookId): Promise<string> => unwrap(commands.rotateSecret(id)),
  /** Sends the newest finished meeting now, once. */
  sendTest: (id: WebhookId): Promise<Delivery> => unwrap(commands.sendTest(id)),
  /** An endpoint's newest deliveries. */
  deliveries: (id: WebhookId): Promise<Delivery[]> => unwrap(commands.deliveries(id)),
};

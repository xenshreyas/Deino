/** The webhooks feature's types: the generated bindings, re-exported. */
export type {
  CreatedWebhook,
  Delivery,
  DeliveryKind,
  DeliveryStatus,
  DisabledReason,
  PayloadFields,
  Webhook,
  WebhookEvent,
  WebhookId,
  WebhookInput,
} from '@/lib/ipc/generated/webhooks';

import type { PayloadFields, WebhookEvent } from '@/lib/ipc/generated/webhooks';

/** Every event, in the order the page lists them. */
export const WEBHOOK_EVENTS = [
  'meeting.ready',
  'notes.generated',
  'notes.edited',
  'action_item.created',
  'action_item.updated',
  'insight.generated',
] as const satisfies readonly WebhookEvent[];

/** Every payload field, in the order the pickers list them. */
export const PAYLOAD_FIELD_KEYS = [
  'notes',
  'actionItems',
  'transcript',
  'insights',
  'attendees',
  'link',
  'audioPath',
] as const satisfies readonly (keyof PayloadFields)[];

/** What a new endpoint sends: the meeting without its transcript or audio path. */
export const DEFAULT_PAYLOAD: PayloadFields = {
  notes: true,
  actionItems: true,
  transcript: false,
  insights: true,
  attendees: true,
  link: true,
  audioPath: false,
};

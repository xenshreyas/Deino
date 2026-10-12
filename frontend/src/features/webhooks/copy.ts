/** Every string Settings → Developer → Webhooks shows. */
import type { DeliveryStatus, DisabledReason, PayloadFields, WebhookEvent } from './types';

export const webhooksCopy = {
  title: 'Webhooks',
  description:
    'Send meetings as signed JSON to Zapier, Make or your own server. Nothing is sent until you add a webhook.',
  add: 'Add webhook',
  empty: 'No webhooks yet.',
  emptyHint: 'In Zapier use “Catch Hook”, in Make “Custom webhook”, and paste its URL here.',
  loadFailed: 'Webhooks could not be loaded.',
  url: 'URL',
  urlPlaceholder: 'https://hooks.zapier.com/hooks/catch/…',
  name: 'Name',
  namePlaceholder: 'Zapier (optional)',
  events: 'Send when',
  noEvents: 'No events: automations and tests only',
  payload: 'Payload includes',
  save: 'Save',
  create: 'Add',
  cancel: 'Cancel',
  edit: 'Edit',
  on: 'On',
  sendTest: 'Send test',
  sending: 'Sending…',
  log: 'Deliveries',
  hideLog: 'Hide deliveries',
  noDeliveries: 'Nothing sent yet.',
  rotate: 'New secret',
  delete: 'Delete',
  confirmDelete: 'Delete this webhook and its delivery log?',
  secretTitle: 'Signing secret',
  secretHint:
    'Copy it now: Deino shows it only once. Verify the webhook-signature header (Standard Webhooks) or x-signature (hex HMAC-SHA256 of the body).',
  copy: 'Copy secret',
  copied: 'Copied',
  dismiss: 'Done',
  tested: (status: string) => `Test ${status}.`,
  attempts: (count: number) => (count === 1 ? '1 attempt' : `${String(count)} attempts`),
  nextTry: (time: string) => `next try ${time}`,
} as const;

/** Why Deino turned an endpoint off, and what to do. */
export const disabledCopy: Record<DisabledReason, string> = {
  failing:
    'Deino turned this webhook off because every delivery failed for a day. Check the URL, then turn it on again.',
  gone: 'Deino turned this webhook off because the endpoint answered 410 Gone. Check the URL, then turn it on again.',
};

/** Event names as the page lists them. */
export const eventCopy: Record<WebhookEvent, string> = {
  'meeting.ready': 'Meeting is ready',
  'notes.generated': 'Notes are generated',
  'notes.edited': 'Notes are edited',
  'action_item.created': 'Action item is added',
  'action_item.updated': 'Action item changes',
  'insight.generated': 'Insights are generated',
};

/** Payload field labels. */
export const fieldCopy: Record<keyof PayloadFields, string> = {
  notes: 'Notes',
  actionItems: 'Action items',
  transcript: 'Transcript',
  insights: 'Insights',
  attendees: 'Attendees',
  link: 'Link',
  audioPath: 'Audio path',
};

/** Delivery states in the log. */
export const statusCopy: Record<DeliveryStatus, string> = {
  pending: 'Retrying',
  sending: 'Sending',
  succeeded: 'Delivered',
  failed: 'Failed',
};

/** What a delivery carried, in the log. */
export function kindLabel(kind: WebhookEvent | 'automation' | 'test'): string {
  if (kind === 'automation') return 'Automation';
  if (kind === 'test') return 'Test';
  return kind;
}

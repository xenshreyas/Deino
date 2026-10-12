import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/webhooks';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import { disabledCopy, webhooksCopy } from '../copy';
import { DEFAULT_PAYLOAD, type Webhook, type WebhookId } from '../types';
import { WebhookSettings } from './webhook-settings';

// Typed against the generated bindings: a Rust signature change breaks this mock.
const commands = vi.hoisted(
  () =>
    ({
      list: vi.fn<typeof Generated.commands.list>(),
      create: vi.fn<typeof Generated.commands.create>(),
      update: vi.fn<typeof Generated.commands.update>(),
      delete: vi.fn<typeof Generated.commands.delete>(),
      rotateSecret: vi.fn<typeof Generated.commands.rotateSecret>(),
      sendTest: vi.fn<typeof Generated.commands.sendTest>(),
      deliveries: vi.fn<typeof Generated.commands.deliveries>(),
    }) satisfies typeof Generated.commands,
);
vi.mock('@/lib/ipc/generated/webhooks', () => ({ commands }));

const ID = '0192f0c0-0000-7000-8000-000000000001' as WebhookId;
const SECRET = 'whsec_c2VjcmV0';

function webhook(patch: Partial<Webhook> = {}): Webhook {
  return {
    id: ID,
    url: 'https://hooks.zapier.com/hooks/catch/1/abc/',
    label: 'Zapier',
    events: ['meeting.ready'],
    fields: DEFAULT_PAYLOAD,
    enabled: true,
    disabledReason: null,
    failureStreak: 0,
    failingSince: null,
    lastSuccessAt: null,
    createdAt: '2026-10-12T09:00:00Z',
    updatedAt: '2026-10-12T09:00:00Z',
    ...patch,
  };
}

function renderPage() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <WebhookSettings />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

describe('WebhookSettings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    commands.deliveries.mockResolvedValue({ status: 'ok', data: [] });
  });

  it('adds a webhook and shows its secret once', async () => {
    commands.list.mockResolvedValue({ status: 'ok', data: [] });
    commands.create.mockResolvedValue({
      status: 'ok',
      data: { webhook: webhook(), secret: SECRET },
    });
    renderPage();

    fireEvent.click(await screen.findByRole('button', { name: webhooksCopy.add }));
    fireEvent.change(screen.getByLabelText(webhooksCopy.url), {
      target: { value: 'https://hooks.zapier.com/hooks/catch/1/abc/' },
    });
    fireEvent.click(screen.getByLabelText('Action item is added'));
    fireEvent.click(screen.getByLabelText('Transcript'));
    fireEvent.click(screen.getByRole('button', { name: webhooksCopy.create }));

    expect(await screen.findByText(SECRET)).toBeTruthy();
    expect(commands.create).toHaveBeenCalledWith({
      url: 'https://hooks.zapier.com/hooks/catch/1/abc/',
      label: '',
      events: ['meeting.ready', 'action_item.created'],
      fields: { ...DEFAULT_PAYLOAD, transcript: true },
      enabled: true,
    });
    fireEvent.click(screen.getByRole('button', { name: webhooksCopy.dismiss }));
    expect(screen.queryByText(SECRET)).toBeNull();
  });

  it('explains why Deino turned a failing webhook off', async () => {
    commands.list.mockResolvedValue({
      status: 'ok',
      data: [webhook({ enabled: false, disabledReason: 'failing' })],
    });
    renderPage();

    expect(await screen.findByText(disabledCopy.failing)).toBeTruthy();
    const toggle = screen.getByRole('switch', { name: webhooksCopy.on });
    expect(toggle.getAttribute('aria-checked')).toBe('false');
  });

  it('sends a test and says how it went', async () => {
    commands.list.mockResolvedValue({ status: 'ok', data: [webhook()] });
    commands.sendTest.mockResolvedValue({
      status: 'ok',
      data: {
        id: '0192f0c0-0000-7000-8000-0000000000aa' as Generated.WebhookDeliveryId,
        webhookId: ID,
        kind: 'test',
        meetingId: 'meeting-1' as Generated.MeetingId,
        status: 'succeeded',
        attempts: 1,
        nextAttemptAt: null,
        lastStatus: 200,
        lastError: null,
        createdAt: '2026-10-12T09:00:00Z',
        finishedAt: '2026-10-12T09:00:00Z',
      },
    });
    renderPage();

    fireEvent.click(await screen.findByRole('button', { name: webhooksCopy.sendTest }));

    expect(await screen.findByText(webhooksCopy.tested('delivered'))).toBeTruthy();
    expect(commands.sendTest).toHaveBeenCalledWith(ID);
  });
});

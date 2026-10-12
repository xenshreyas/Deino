import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { asMeetingId } from '@/features/meetings';
import type * as Generated from '@/lib/ipc/generated/templates';
import { createQueryClient } from '@/lib/ipc';

import type { Template, TemplateChoice, TemplateId } from '../types';
import { MeetingTemplateMenu } from './meeting-template-picker';

const commands = vi.hoisted(
  () =>
    ({
      list: vi.fn<typeof Generated.commands.list>(),
      get: vi.fn<typeof Generated.commands.get>(),
      create: vi.fn<typeof Generated.commands.create>(),
      update: vi.fn<typeof Generated.commands.update>(),
      duplicate: vi.fn<typeof Generated.commands.duplicate>(),
      delete: vi.fn<typeof Generated.commands.delete>(),
      importJson: vi.fn<typeof Generated.commands.importJson>(),
      exportJson: vi.fn<typeof Generated.commands.exportJson>(),
      preview: vi.fn<typeof Generated.commands.preview>(),
      meetingTemplate: vi.fn<typeof Generated.commands.meetingTemplate>(),
      setMeetingTemplate: vi.fn<typeof Generated.commands.setMeetingTemplate>(),
    }) satisfies typeof Generated.commands,
);
vi.mock('@/lib/ipc/generated/templates', () => ({ commands }));

function template(id: string, name: string): Template {
  return {
    id: id as TemplateId,
    isBuiltin: true,
    content: { name, description: '', meetingContext: '', sections: [], autoRules: [] },
    createdAt: '2026-10-01T09:00:00Z',
    updatedAt: '2026-10-01T09:00:00Z',
  };
}

const discovery = template('sales_discovery', 'Sales discovery');
const circleback = template('circleback_style', 'Circleback style');
const byRule: TemplateChoice = {
  template: discovery,
  reason: { kind: 'rule', rule: 'external discovery call' },
};
const MEETING = asMeetingId('meeting-fixture-sales-discovery');

function renderMenu(onSwitched: (name: string) => void = vi.fn()) {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <MeetingTemplateMenu meetingId={MEETING} onSwitched={onSwitched} />
    </QueryClientProvider>,
  );
}

/** Opens a Radix menu the way a pointer does. */
function open(trigger: HTMLElement) {
  fireEvent.pointerDown(trigger, { button: 0, ctrlKey: false });
}

describe('the meeting template picker', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    commands.list.mockResolvedValue({ status: 'ok', data: [circleback, discovery] });
    commands.meetingTemplate.mockResolvedValue({ status: 'ok', data: byRule });
    commands.setMeetingTemplate.mockResolvedValue({
      status: 'ok',
      data: { template: circleback, reason: { kind: 'chosen' } },
    });
  });

  test('names the template quietly; why it was chosen is the menu header (UX-06)', async () => {
    renderMenu();

    const trigger = await screen.findByRole('button', { name: 'Note template' });
    expect(trigger.textContent).toBe('Sales discovery');
    expect(screen.queryByText('Matched rule: external discovery call')).toBeNull();
    open(trigger);
    expect(await screen.findByText('Matched rule: external discovery call')).toBeDefined();
  });

  test('switching saves the choice and reports the new template', async () => {
    const onSwitched = vi.fn<(name: string) => void>();
    renderMenu(onSwitched);

    open(await screen.findByRole('button', { name: 'Note template' }));
    fireEvent.click(await screen.findByRole('menuitem', { name: /Circleback style/ }));

    await waitFor(() => {
      expect(commands.setMeetingTemplate).toHaveBeenCalledWith(MEETING, 'circleback_style');
    });
    await waitFor(() => {
      expect(onSwitched).toHaveBeenCalledWith('Circleback style');
    });
  });
});

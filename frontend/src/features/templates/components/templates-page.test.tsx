import { QueryClientProvider } from '@tanstack/react-query';
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from '@tanstack/react-router';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { settingsSearchSchema } from '@/features/settings/search';
import type * as Generated from '@/lib/ipc/generated/templates';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import type { Template, TemplateId } from '../types';
import { TemplatesPage } from './templates-page';

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
vi.mock('@/lib/ipc/generated/meetings', () => ({
  commands: { list: () => Promise.resolve({ status: 'ok', data: [] }) },
}));

function template(id: string, isBuiltin: boolean, sections: string[]): Template {
  return {
    id: id as TemplateId,
    isBuiltin,
    content: {
      name: isBuiltin ? 'General' : 'Board prep',
      description: '',
      meetingContext: '',
      sections: sections.map((title) => ({ title, instructions: '', format: 'bullets' })),
      autoRules: [],
    },
    createdAt: '2026-10-01T09:00:00Z',
    updatedAt: '2026-10-01T09:00:00Z',
  };
}

const mine = template('mine', false, ['Decisions', 'Risks', 'Asks']);
const general = template('general', true, ['Overview']);

/** The page at `/settings?section=templates&template=<id>`. */
function renderEditor(id: string) {
  const root = createRootRoute();
  const settings = createRoute({
    getParentRoute: () => root,
    path: '/settings',
    validateSearch: settingsSearchSchema,
    component: TemplatesPage,
  });
  const router = createRouter({
    routeTree: root.addChildren([settings]),
    history: createMemoryHistory({
      initialEntries: [`/settings?section=templates&template=${id}`],
    }),
  });
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <RouterProvider router={router} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

describe('the template editor', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    commands.get.mockImplementation((id) =>
      Promise.resolve({ status: 'ok', data: id === 'general' ? general : mine }),
    );
    commands.update.mockImplementation((id, content) =>
      Promise.resolve({ status: 'ok', data: { ...mine, id, content } }),
    );
  });

  test('reorders sections from the keyboard and saves them in the new order', async () => {
    renderEditor('mine');

    const handle = await screen.findByRole('button', { name: 'Reorder Decisions' });
    fireEvent.keyDown(handle, { key: 'ArrowDown' });
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => {
      expect(commands.update).toHaveBeenCalledTimes(1);
    });
    const titles = commands.update.mock.calls[0]?.[1].sections.map(({ title }) => title);
    expect(titles).toEqual(['Risks', 'Decisions', 'Asks']);
  });

  test('changes a section format and removes a section before saving', async () => {
    renderEditor('mine');

    const risks = await screen.findByRole('listitem', { name: 'Risks' });
    fireEvent.click(within(risks).getByRole('button', { name: 'Remove section' }));
    fireEvent.change(screen.getByRole('textbox', { name: 'Instructions: Asks' }), {
      target: { value: 'What we need from the board' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => {
      expect(commands.update).toHaveBeenCalledTimes(1);
    });
    const sections = commands.update.mock.calls[0]?.[1].sections;
    expect(sections?.map(({ title }) => title)).toEqual(['Decisions', 'Asks']);
    expect(sections?.[1]?.instructions).toBe('What we need from the board');
  });

  test('shows built-ins read-only, offering a copy instead of Save', async () => {
    renderEditor('general');

    expect(await screen.findByText('Built-in templates are read-only')).toBeDefined();
    expect(screen.queryByRole('button', { name: 'Save' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Reorder Overview' })).toBeNull();
    expect(screen.getByRole('button', { name: 'Duplicate' })).toBeDefined();
  });

  test('shows a rejected save inline, as Rust worded it', async () => {
    commands.update.mockResolvedValue({
      status: 'error',
      error: { code: 'invalid', message: 'section 2 repeats an earlier title' },
    });
    renderEditor('mine');

    fireEvent.click(await screen.findByRole('button', { name: 'Save' }));

    expect((await screen.findByRole('alert')).textContent).toBe(
      'section 2 repeats an earlier title',
    );
  });
});

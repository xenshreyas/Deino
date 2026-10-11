import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/vocabulary';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import type { VocabTerm, VocabTermId } from '../types';
import { VocabularySettings } from './vocabulary-settings';

const commands = vi.hoisted(
  () =>
    ({
      list: vi.fn<typeof Generated.commands.list>(),
      create: vi.fn<typeof Generated.commands.create>(),
      update: vi.fn<typeof Generated.commands.update>(),
      delete: vi.fn<typeof Generated.commands.delete>(),
      corrections: vi.fn<typeof Generated.commands.corrections>(),
      undo: vi.fn<typeof Generated.commands.undo>(),
    }) satisfies typeof Generated.commands,
);
vi.mock('@/lib/ipc/generated/vocabulary', () => ({ commands }));

function entry(id: string, kind: VocabTerm['kind'], text: string, soundsLike: string[]): VocabTerm {
  return {
    id: id as VocabTermId,
    kind,
    text,
    soundsLike,
    createdAt: '2026-10-11T09:00:00Z',
    updatedAt: '2026-10-11T09:00:00Z',
  };
}

const shreyas = entry('term-1', 'term', 'Shreyas', ['Sreyas']);
const gonna = entry('term-2', 'replacement', 'going to', ['gonna']);

function renderSettings() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <VocabularySettings />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

describe('Settings → Vocabulary', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    commands.list.mockResolvedValue({ status: 'ok', data: [gonna, shreyas] });
    commands.create.mockImplementation((input) =>
      Promise.resolve({
        status: 'ok',
        data: { ...shreyas, ...input, id: 'term-3' as VocabTermId },
      }),
    );
    commands.delete.mockResolvedValue({ status: 'ok', data: null });
  });

  test('terms show what they sound like and replacements what they replace', async () => {
    renderSettings();

    const terms = await screen.findByRole('group', { name: 'Words and names' });
    expect(within(terms).getByText('Shreyas')).toBeDefined();
    expect(within(terms).getByText('Sounds like Sreyas')).toBeDefined();
    const replacements = screen.getByRole('group', { name: 'Replacements' });
    expect(within(replacements).getByText('gonna')).toBeDefined();
    expect(within(replacements).getByText('going to')).toBeDefined();
  });

  test('a new term is saved with its comma-separated spellings', async () => {
    renderSettings();
    const terms = await screen.findByRole('group', { name: 'Words and names' });

    fireEvent.change(within(terms).getByLabelText('Word or name'), {
      target: { value: ' Deino ' },
    });
    fireEvent.change(within(terms).getByLabelText('Sounds like'), {
      target: { value: 'Dino,  Dyno ,' },
    });
    fireEvent.click(within(terms).getByRole('button', { name: 'Add' }));

    await waitFor(() => {
      expect(commands.create).toHaveBeenCalledWith({
        kind: 'term',
        text: 'Deino',
        soundsLike: ['Dino', 'Dyno'],
      });
    });
  });

  test('a replacement cannot be added without something to replace, and an entry can be removed', async () => {
    renderSettings();
    const replacements = await screen.findByRole('group', { name: 'Replacements' });

    fireEvent.change(within(replacements).getByLabelText('With'), { target: { value: 'want to' } });
    expect(within(replacements).getByRole('button', { name: 'Add' })).toHaveProperty(
      'disabled',
      true,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Remove Shreyas' }));

    await waitFor(() => {
      expect(commands.delete).toHaveBeenCalledWith('term-1');
    });
  });
});

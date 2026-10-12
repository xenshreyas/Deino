import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/stats';
import { createQueryClient } from '@/lib/ipc';

import type { LlmUsage, RecapStatus } from '../types';
import { ActivityHeatmap } from './activity-heatmap';
import { AudienceBar } from './audience-bar';
import { LlmUsageTable } from './llm-usage-table';
import { RecapClosedLine, YearRecapSection } from './year-recap';

const commands = vi.hoisted(
  () =>
    ({
      overview: vi.fn<typeof Generated.commands.overview>(),
      heatmap: vi.fn<typeof Generated.commands.heatmap>(),
      recap: vi.fn<typeof Generated.commands.recap>(),
      writeRecap: vi.fn<typeof Generated.commands.writeRecap>(),
    }) satisfies typeof Generated.commands,
);
vi.mock('@/lib/ipc/generated/stats', () => ({ commands }));

const RECAP = {
  year: 2026,
  markdown: 'A year of standups.\n\n### People\nMaya.',
  generatedAt: '2026-12-03T09:00:00Z',
};

function withQueries(node: ReactNode) {
  return render(<QueryClientProvider client={createQueryClient()}>{node}</QueryClientProvider>);
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe('year in review', () => {
  test('before December only a quiet line says when it opens', () => {
    const closed: RecapStatus = { year: 2026, available: false, recap: null };

    withQueries(
      <>
        <YearRecapSection status={closed} />
        <RecapClosedLine status={closed} />
      </>,
    );

    expect(screen.getByText('Your 2026 in review opens in December.')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Write my year in review' })).toBeNull();
  });

  test('in December it can be written, and asks the stats plugin once', async () => {
    commands.writeRecap.mockResolvedValue({ status: 'ok', data: RECAP });
    withQueries(<YearRecapSection status={{ year: 2026, available: true, recap: null }} />);

    fireEvent.click(screen.getByRole('button', { name: 'Write my year in review' }));

    await waitFor(() => {
      expect(commands.writeRecap).toHaveBeenCalledTimes(1);
    });
  });

  test('a written recap is shown as Markdown with its date', () => {
    withQueries(<YearRecapSection status={{ year: 2026, available: true, recap: RECAP }} />);

    expect(screen.getByRole('heading', { name: 'People' })).toBeTruthy();
    expect(screen.getByText('Written Dec 3, 2026')).toBeTruthy();
  });
});

describe('charts and tables', () => {
  test('the audience legend names every part with its count and share', () => {
    render(<AudienceBar audience={{ internal: 4, external: 4, unknown: 1 }} />);

    expect(screen.getByText('Internal').parentElement?.textContent).toBe('Internal444%');
    expect(screen.getByText('Unknown').parentElement?.textContent).toBe('Unknown111%');
  });

  test('LLM usage says the cost is an estimate and which calls are unpriced', () => {
    const usage: LlmUsage = {
      models: [
        {
          provider: 'openai',
          model: 'gpt-5',
          calls: 1,
          failed: 0,
          inputTokens: 1000,
          outputTokens: 100,
          estimatedCostMicros: null,
        },
      ],
      estimatedCostMicros: 0,
      unpricedCalls: 1,
    };

    render(<LlmUsageTable usage={usage} />);

    expect(screen.getByRole('cell', { name: '—' })).toBeTruthy();
    expect(screen.getByText(/Costs are estimates/).textContent).toContain(
      '1 call to models without a known price',
    );
  });

  test('every heatmap day is labelled, and hovering reads it out', () => {
    render(
      <ActivityHeatmap
        heatmap={{
          firstDay: '2026-09-28',
          lastDay: '2026-10-04',
          days: [{ day: '2026-09-29', meetings: 2 }],
        }}
      />,
    );

    const day = screen.getByRole('img', { name: 'Tue, Sep 29, 2026 · 2 meetings' });
    fireEvent.pointerEnter(day);

    expect(screen.getAllByRole('img')).toHaveLength(7);
    expect(screen.getByText('Tue, Sep 29, 2026 · 2 meetings', { selector: 'p' })).toBeTruthy();
  });
});

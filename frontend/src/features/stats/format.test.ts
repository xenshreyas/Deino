import { describe, expect, test } from 'vitest';

import { formatCost, formatCount, formatDay, formatHours, hourTick, percent } from './format';
import { heatGrid, levelOf } from './heatmap-grid';

describe('format', () => {
  test('counts compact past ten thousand', () => {
    expect(formatCount(1284)).toBe('1,284');
    expect(formatCount(12_900)).toBe('12.9K');
    expect(formatCount(4_200_000)).toBe('4.2M');
  });

  test('hours keep a decimal only while small', () => {
    expect(formatHours(0)).toBe('0');
    expect(formatHours(14_903)).toBe('4.1');
    expect(formatHours(889_200)).toBe('247');
  });

  test('estimated dollars from millionths', () => {
    expect(formatCost(0)).toBe('$0');
    expect(formatCost(4_000)).toBe('<$0.01');
    expect(formatCost(2_110_000)).toBe('$2.11');
    expect(formatCost(1_234_000_000)).toBe('$1.2K');
  });

  test('days, hour ticks and shares', () => {
    expect(formatDay('2026-09-29')).toBe('Tue, Sep 29, 2026');
    expect([0, 6, 12, 18].map(hourTick)).toEqual(['12a', '6a', '12p', '6p']);
    expect(percent(1, 3)).toBe(33);
    expect(percent(0, 0)).toBe(0);
  });
});

describe('heatmap grid', () => {
  test('levels scale to the busiest day in four steps', () => {
    expect([0, 1, 2, 3, 4, 8].map((count) => levelOf(count, 8))).toEqual([0, 1, 1, 2, 2, 4]);
    expect(levelOf(1, 1)).toBe(4);
  });

  test('weeks run Monday to Sunday and stop at today', () => {
    const grid = heatGrid({
      firstDay: '2026-09-28',
      lastDay: '2026-10-11',
      days: [
        { day: '2026-09-29', meetings: 2 },
        { day: '2026-10-01', meetings: 1 },
      ],
    });

    expect(grid.weeks).toHaveLength(2);
    expect(grid.weeks[0]?.[1]).toEqual({ day: '2026-09-29', meetings: 2, level: 4 });
    expect(grid.weeks[0]?.[3]).toEqual({ day: '2026-10-01', meetings: 1, level: 2 });
    expect(grid.weeks[1]?.[6]?.day).toBe('2026-10-11');
    expect(grid.months).toEqual([]);
  });

  test('a month is labelled at the first week holding its 1st', () => {
    const grid = heatGrid({ firstDay: '2026-09-21', lastDay: '2026-10-11', days: [] });

    expect(grid.months).toEqual([{ week: 1, label: 'Oct' }]);
  });
});

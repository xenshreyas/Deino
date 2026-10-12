/**
 * The heatmap as a grid: one column per week (Monday first), one row per
 * weekday, each cell shaded by one of four levels of a single hue. Pure.
 */
import { parseDay } from './format';
import type { Heatmap } from './types';

/** Shades above "none"; the scale is one hue, light to dark (§ dataviz). */
export const LEVELS = 4;

const DAY_MS = 86_400_000;
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/** One day of the grid. */
export interface HeatCell {
  /** `YYYY-MM-DD`. */
  day: string;
  meetings: number;
  /** 0 for none, else 1–{@link LEVELS}. */
  level: number;
}

/** The grid: weeks of seven cells (`null` past today) and month labels. */
export interface HeatGrid {
  weeks: (HeatCell | null)[][];
  /** A month's short name at the first week that holds its 1st. */
  months: { week: number; label: string }[];
}

/** The shade of `meetings` on a scale up to `busiest`. */
export function levelOf(meetings: number, busiest: number): number {
  if (meetings <= 0 || busiest <= 0) return 0;
  return Math.min(LEVELS, Math.max(1, Math.ceil((meetings / busiest) * LEVELS)));
}

function isoDay(date: Date): string {
  return date.toISOString().slice(0, 10);
}

/** Lays `heatmap` out as weeks, from its first Monday to today. */
export function heatGrid(heatmap: Heatmap): HeatGrid {
  const counts = new Map(heatmap.days.map((entry) => [entry.day, entry.meetings]));
  const busiest = Math.max(0, ...counts.values());
  const first = parseDay(heatmap.firstDay).getTime();
  const last = parseDay(heatmap.lastDay).getTime();
  const weeks: (HeatCell | null)[][] = [];
  const months: HeatGrid['months'] = [];
  for (let start = first, week = 0; start <= last; start += 7 * DAY_MS, week += 1) {
    const cells = Array.from({ length: 7 }, (_, offset): HeatCell | null => {
      const time = start + offset * DAY_MS;
      if (time > last) return null;
      const day = isoDay(new Date(time));
      const meetings = counts.get(day) ?? 0;
      return { day, meetings, level: levelOf(meetings, busiest) };
    });
    const firstOfMonth = cells.find((cell) => cell?.day.endsWith('-01') === true);
    if (firstOfMonth !== undefined && firstOfMonth !== null && week > 0) {
      months.push({ week, label: MONTHS[parseDay(firstOfMonth.day).getUTCMonth()] ?? '' });
    }
    weeks.push(cells);
  }
  return { weeks, months };
}

/**
 * Pure helpers behind the insight views: how a typed value reads, the
 * table's columns and rows, and how a column sorts. Kept out of components
 * so they are tested on their own.
 */
import type {
  FieldKind,
  FieldValue,
  InsightAnswer,
  InsightDef,
  InsightTable,
  MeetingId,
  RunProgress,
} from './types';

/** `120`, `4.5`, `Yes`, `Oct 8, 2026`, or the text; empty for no value. */
export function formatValue(value: FieldValue | null): string {
  if (value === null) return '';
  switch (value.type) {
    case 'number':
      return Number.isInteger(value.value)
        ? value.value.toLocaleString('en-US')
        : String(value.value);
    case 'boolean':
      return value.value ? 'Yes' : 'No';
    case 'date':
      return formatDay(value.value);
    case 'text':
    case 'choice':
      return value.value;
  }
}

/** `2026-10-08` as `Oct 8, 2026`. */
export function formatDay(day: string): string {
  const date = new Date(`${day}T12:00:00`);
  if (Number.isNaN(date.getTime())) return day;
  return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
}

/** A recording offset as `04:31`, or `1:04:31` from the first hour. */
export function formatMoment(ms: number): string {
  const total = Math.floor(ms / 1000);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor(total / 60) % 60;
  const seconds = String(total % 60).padStart(2, '0');
  if (hours > 0) return `${String(hours)}:${String(minutes).padStart(2, '0')}:${seconds}`;
  return `${String(minutes).padStart(2, '0')}:${seconds}`;
}

/** How a column's values compare. */
export type ColumnType = 'text' | 'number' | 'boolean' | 'date' | 'meeting-date' | 'moment';

/** One column of an insight's table. */
export interface Column {
  /** Stable key: `meeting`, `date`, `said`, `answer` or `field:<name>`. */
  key: string;
  label: string;
  type: ColumnType;
}

const COLUMN_TYPES: Record<FieldKind['type'], ColumnType> = {
  text: 'text',
  number: 'number',
  boolean: 'boolean',
  date: 'date',
  enum: 'text',
};

/** The meeting, its date, one column per field (or the answer), who said it. */
export function columnsOf(def: InsightDef): Column[] {
  const values: Column[] =
    def.fields.length === 0
      ? [{ key: 'answer', label: 'Answer', type: 'text' }]
      : def.fields.map((field) => ({
          key: `field:${field.name}`,
          label: field.name,
          type: COLUMN_TYPES[field.kind.type],
        }));
  return [
    { key: 'meeting', label: 'Meeting', type: 'text' },
    { key: 'date', label: 'Date', type: 'meeting-date' },
    ...values,
    { key: 'said', label: 'Said', type: 'moment' },
  ];
}

/** One answer of the table, with its meeting. */
export interface Row {
  /** Unique within the table. */
  id: string;
  meetingId: MeetingId;
  meetingTitle: string;
  startedAt: string;
  answer: InsightAnswer;
  /** True when the answer predates the insight's latest edit. */
  stale: boolean;
}

/** One row per answer, newest meeting first (the order Rust returns). */
export function rowsOf(table: InsightTable): Row[] {
  return table.rows.flatMap((row) =>
    row.run.answers.map((answer, index) => ({
      id: `${row.run.meetingId}:${String(index)}`,
      meetingId: row.run.meetingId,
      meetingTitle: row.meetingTitle,
      startedAt: row.meetingStartedAt,
      answer,
      stale: row.run.revision !== null && row.run.revision < table.def.revision,
    })),
  );
}

/** The typed value of `row` under `column`, for display and sorting. */
export function cellValue(row: Row, column: Column): FieldValue | null {
  const value = row.answer.value;
  if (column.key === 'answer' && value.kind === 'text') return { type: 'text', value: value.text };
  if (!column.key.startsWith('field:') || value.kind !== 'fields') return null;
  const name = column.key.slice('field:'.length);
  return value.fields.find((entry) => entry.name === name)?.value ?? null;
}

/** A sortable key: numbers, booleans and dates as numbers, the rest as text. */
function sortKey(row: Row, column: Column): number | string | null {
  if (column.key === 'meeting') return row.meetingTitle.toLowerCase();
  if (column.key === 'date') return Date.parse(row.startedAt);
  if (column.key === 'said') return row.answer.timestampMs;
  const value = cellValue(row, column);
  if (value === null) return null;
  if (value.type === 'number') return value.value;
  if (value.type === 'boolean') return value.value ? 1 : 0;
  if (value.type === 'date') return Date.parse(value.value);
  return value.value.toLowerCase();
}

/** Which column the table sorts by, and which way. */
export interface Sort {
  key: string;
  direction: 'asc' | 'desc';
}

/** `rows` sorted by `sort`; rows without a value always last. */
export function sortRows(rows: readonly Row[], columns: readonly Column[], sort: Sort): Row[] {
  const column = columns.find((candidate) => candidate.key === sort.key);
  if (column === undefined) return [...rows];
  const sign = sort.direction === 'asc' ? 1 : -1;
  return [...rows].sort((a, b) => {
    const [x, y] = [sortKey(a, column), sortKey(b, column)];
    if (x === null || y === null) return x === y ? 0 : x === null ? 1 : -1;
    if (x < y) return -sign;
    return x > y ? sign : 0;
  });
}

/** The next sort after clicking `key`: a new column starts descending for dates and numbers. */
export function nextSort(current: Sort, column: Column): Sort {
  if (current.key === column.key) {
    return { key: column.key, direction: current.direction === 'asc' ? 'desc' : 'asc' };
  }
  const startsHigh = column.type !== 'text';
  return { key: column.key, direction: startsHigh ? 'desc' : 'asc' };
}

/** How many meetings a run covers and how many are still waiting. */
export function runState(progress: RunProgress): { total: number; waiting: number } {
  return {
    total: progress.pending + progress.done + progress.failed,
    waiting: progress.pending,
  };
}

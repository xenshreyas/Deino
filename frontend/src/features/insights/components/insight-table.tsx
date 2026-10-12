/**
 * One insight's answers across meetings as a table (Circleback §10.3): a
 * row per answer, a column per field, sortable headers, and rows that open
 * to show long values in full. Scrolls sideways when the fields outgrow the
 * page.
 */
import { Link } from '@tanstack/react-router';
import { useState } from 'react';

import { cn } from '@/ui/cn';
import { ArrowDown, ArrowUp, ChevronRight, ICON_PROPS } from '@/ui/icons';

import { insightsCopy } from '../copy';
import {
  cellValue,
  type Column,
  formatValue,
  nextSort,
  type Row,
  type Sort,
  sortRows,
} from '../model';
import { Attribution } from './answer-view';

/** Props of {@link InsightTable}. */
export interface InsightTableProps {
  name: string;
  columns: readonly Column[];
  rows: readonly Row[];
}

const DEFAULT_SORT: Sort = { key: 'date', direction: 'desc' };

/** The table, sorted newest meeting first until a header is clicked. */
export function InsightTable({ name, columns, rows }: InsightTableProps) {
  const [sort, setSort] = useState<Sort>(DEFAULT_SORT);
  const [open, setOpen] = useState<ReadonlySet<string>>(new Set());
  const toggle = (id: string) => {
    const next = new Set(open);
    if (!next.delete(id)) next.add(id);
    setOpen(next);
  };
  return (
    <div className="-mx-3 overflow-x-auto px-3 pb-2">
      <table
        aria-label={insightsCopy.table.label(name)}
        className="w-full min-w-full border-collapse text-sm"
      >
        <HeaderRow columns={columns} sort={sort} onSort={setSort} />
        <tbody>
          {sortRows(rows, columns, sort).map((row) => (
            <TableRow
              key={row.id}
              row={row}
              columns={columns}
              open={open.has(row.id)}
              onToggle={() => {
                toggle(row.id);
              }}
            />
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** The header row; clicking a header sorts by it, again reverses. */
function HeaderRow({
  columns,
  sort,
  onSort,
}: {
  columns: readonly Column[];
  sort: Sort;
  onSort: (sort: Sort) => void;
}) {
  return (
    <thead>
      <tr className="border-b border-border">
        {columns.map((column) => (
          <HeaderCell
            key={column.key}
            column={column}
            sort={sort}
            onSort={() => {
              onSort(nextSort(sort, column));
            }}
          />
        ))}
      </tr>
    </thead>
  );
}

/** A sortable column header. */
function HeaderCell({ column, sort, onSort }: { column: Column; sort: Sort; onSort: () => void }) {
  const active = sort.key === column.key;
  const Arrow = sort.direction === 'asc' ? ArrowUp : ArrowDown;
  return (
    <th
      scope="col"
      aria-sort={active ? (sort.direction === 'asc' ? 'ascending' : 'descending') : 'none'}
      className={cn('h-9 px-3 text-left align-middle', isNumeric(column) && 'text-right')}
    >
      <button
        type="button"
        onClick={onSort}
        aria-label={insightsCopy.table.sortBy(column.label)}
        className={cn(
          'inline-flex items-center gap-1 rounded-xs text-xs font-semibold whitespace-nowrap',
          'transition-colors duration-(--dur-fast) ease-out hover:text-fg',
          'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus-ring',
          active ? 'text-fg' : 'text-fg-muted',
        )}
      >
        {column.label}
        <Arrow {...ICON_PROPS} size={12} className={cn(!active && 'invisible')} />
      </button>
    </th>
  );
}

function isNumeric(column: Column): boolean {
  return column.type === 'number';
}

/** One answer; clicking it shows long values in full. */
function TableRow({
  row,
  columns,
  open,
  onToggle,
}: {
  row: Row;
  columns: readonly Column[];
  open: boolean;
  onToggle: () => void;
}) {
  return (
    <tr
      onClick={onToggle}
      title={row.stale ? insightsCopy.table.stale : undefined}
      className={cn(
        'group cursor-default border-b border-border align-top transition-colors duration-(--dur-fast) ease-out last:border-b-0 hover:bg-bg-subtle',
        row.stale && 'text-fg-muted',
      )}
    >
      {columns.map((column, index) => (
        <td
          key={column.key}
          className={cn('px-3 py-2.5', isNumeric(column) && 'text-right tabular-nums')}
        >
          {index === 0 ? (
            <div className="-ml-6 flex items-start gap-1">
              <Disclosure open={open} onToggle={onToggle} />
              <Cell row={row} column={column} open={open} />
            </div>
          ) : (
            <Cell row={row} column={column} open={open} />
          )}
        </td>
      ))}
    </tr>
  );
}

/** The row's open/close control, for keyboards and screen readers. */
function Disclosure({ open, onToggle }: { open: boolean; onToggle: () => void }) {
  return (
    <button
      type="button"
      aria-expanded={open}
      aria-label={open ? insightsCopy.table.collapse : insightsCopy.table.expand}
      onClick={(event) => {
        event.stopPropagation();
        onToggle();
      }}
      className="flex size-5 shrink-0 items-center justify-center rounded-xs text-fg-subtle opacity-0 transition-[opacity,transform] duration-(--dur-fast) ease-out group-hover:opacity-100 hover:text-fg focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-focus-ring aria-expanded:opacity-100"
    >
      <ChevronRight
        {...ICON_PROPS}
        size={14}
        className={cn('transition-transform duration-(--dur-fast)', open && 'rotate-90')}
      />
    </button>
  );
}

/** A cell's content: clipped to one line until the row opens. */
function Cell({ row, column, open }: { row: Row; column: Column; open: boolean }) {
  const clip = open ? 'whitespace-pre-line' : 'block max-w-[18rem] truncate';
  switch (column.key) {
    case 'meeting':
      return (
        <Link
          to="/meetings/$meetingId"
          params={{ meetingId: row.meetingId }}
          search={{ tab: 'notes' }}
          onClick={(event) => {
            event.stopPropagation();
          }}
          className={cn(
            'block max-w-[14rem] min-w-0 text-fg underline-offset-4 hover:underline',
            open ? 'whitespace-normal' : 'truncate',
          )}
        >
          {row.meetingTitle}
        </Link>
      );
    case 'date':
      return (
        <span className="whitespace-nowrap text-fg-muted tabular-nums">
          {shortDate(row.startedAt)}
        </span>
      );
    case 'said':
      return <Attribution meetingId={row.meetingId} answer={row.answer} />;
    default:
      return (
        <span className={cn(clip, 'text-inherit')}>{formatValue(cellValue(row, column))}</span>
      );
  }
}

/** `Oct 8`, with the year when it is not this year. */
function shortDate(iso: string): string {
  const date = new Date(iso);
  const sameYear = date.getFullYear() === new Date().getFullYear();
  return date.toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    ...(sameYear ? {} : { year: 'numeric' }),
  });
}

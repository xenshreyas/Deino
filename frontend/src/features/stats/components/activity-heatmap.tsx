/**
 * The GitHub-style activity heatmap: a week per column over the last 12
 * months, one hue in four steps (the accent, washed into the canvas, so
 * both themes keep "more = stronger"). Hovering or focusing a day reads it
 * out below the grid; every cell also carries its own accessible label.
 */
import { useMemo, useState } from 'react';

import { cn } from '@/ui/cn';

import { statsCopy } from '../copy';
import { formatDay } from '../format';
import { type HeatCell, type HeatGrid, heatGrid } from '../heatmap-grid';
import type { Heatmap } from '../types';

const CELL = 11;
const GAP = 3;
const STEP = CELL + GAP;
const LEFT = 32;
const TOP = 20;
/** Weekday labels on rows 0, 2 and 4 (Mon, Wed, Fri), as GitHub does. */
const ROW_LABELS = [
  [0, 'Mon'],
  [2, 'Wed'],
  [4, 'Fri'],
] as const;

/** The fill of each level: empty, then the accent at four strengths. */
const LEVEL_FILL = [
  'fill-bg-muted',
  'fill-accent/25',
  'fill-accent/50',
  'fill-accent/75',
  'fill-accent',
] as const;

function Cell({
  cell,
  x,
  y,
  onShow,
}: {
  cell: HeatCell;
  x: number;
  y: number;
  onShow: (cell: HeatCell) => void;
}) {
  const label = statsCopy.dayReadout(formatDay(cell.day), cell.meetings);
  return (
    <rect
      x={x}
      y={y}
      width={CELL}
      height={CELL}
      rx={2}
      role="img"
      aria-label={label}
      className={cn(LEVEL_FILL[cell.level], 'outline-none hover:stroke-fg hover:stroke-1')}
      onPointerEnter={() => {
        onShow(cell);
      }}
    />
  );
}

/** "Less ▢▢▢▢▢ More". */
function Legend() {
  return (
    <div className="flex items-center gap-1 text-xs text-fg-subtle" aria-hidden>
      <span className="mr-1">{statsCopy.less}</span>
      {LEVEL_FILL.map((fill) => (
        <svg key={fill} width={CELL} height={CELL}>
          <rect width={CELL} height={CELL} rx={2} className={fill} />
        </svg>
      ))}
      <span className="ml-1">{statsCopy.more}</span>
    </div>
  );
}

/** Month labels over the columns and weekday labels beside the rows. */
function Axes({ grid }: { grid: HeatGrid }) {
  return (
    <>
      {grid.months.map(({ week, label }) => (
        <text key={week} x={LEFT + week * STEP} y={12} className="fill-fg-subtle text-xs">
          {label}
        </text>
      ))}
      {ROW_LABELS.map(([row, label]) => (
        <text key={label} x={0} y={TOP + row * STEP + 10} className="fill-fg-subtle text-xs">
          {label}
        </text>
      ))}
    </>
  );
}

/** The grid itself; `onShow(null)` when the pointer leaves it. */
function Grid({ grid, onShow }: { grid: HeatGrid; onShow: (cell: HeatCell | null) => void }) {
  return (
    <svg
      width={LEFT + grid.weeks.length * STEP}
      height={TOP + 7 * STEP}
      role="group"
      aria-label={statsCopy.activityHint}
      className="block"
      onPointerLeave={() => {
        onShow(null);
      }}
    >
      <Axes grid={grid} />
      {grid.weeks.map((week, column) =>
        week.map((cell, row) =>
          cell === null ? null : (
            <Cell
              key={cell.day}
              cell={cell}
              x={LEFT + column * STEP}
              y={TOP + row * STEP}
              onShow={onShow}
            />
          ),
        ),
      )}
    </svg>
  );
}

/** Props of {@link ActivityHeatmap}. */
export interface ActivityHeatmapProps {
  heatmap: Heatmap;
}

/** The last year of meetings, one square per day. */
export function ActivityHeatmap({ heatmap }: ActivityHeatmapProps) {
  const grid = useMemo(() => heatGrid(heatmap), [heatmap]);
  const [shown, setShown] = useState<HeatCell | null>(null);
  return (
    <div className="flex flex-col gap-2">
      <div className="overflow-x-auto">
        <Grid grid={grid} onShow={setShown} />
      </div>
      <div className="flex h-5 items-center justify-between gap-4">
        <p className="text-xs text-fg-muted tabular-nums" aria-live="polite">
          {shown === null ? null : statsCopy.dayReadout(formatDay(shown.day), shown.meetings)}
        </p>
        <Legend />
      </div>
    </div>
  );
}

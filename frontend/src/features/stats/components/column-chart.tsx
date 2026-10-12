/**
 * A small column chart for one series (meetings per weekday or per hour):
 * thin columns on a hairline baseline, 4px rounded tops, the busiest column
 * in the accent and the rest in a recessive grey (emphasis, not value
 * colouring). Hovering or focusing a column reads its value out above.
 */
import { useState } from 'react';

import { cn } from '@/ui/cn';

import { statsCopy } from '../copy';

/** Props of {@link ColumnChart}. */
export interface ColumnChartProps {
  /** Accessible name of the chart. */
  label: string;
  values: readonly number[];
  /** The name of slot `index`, as the readout says it. */
  name: (index: number) => string;
  /** The tick under slot `index`, or `null` for none. */
  tick: (index: number) => string | null;
}

const HEIGHT_PX = 72;

/** One column: a hit target the full height of the chart, the bar inside. */
function Column({
  value,
  busiest,
  label,
  onShow,
}: {
  value: number;
  busiest: number;
  label: string;
  onShow: () => void;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      className="group flex h-full min-w-0 flex-1 cursor-default items-end justify-center outline-none"
      onPointerEnter={onShow}
      onFocus={onShow}
    >
      <span
        className={cn(
          'w-full max-w-6 rounded-t-xs transition-colors duration-120',
          value === busiest && busiest > 0 ? 'bg-accent' : 'bg-fg-subtle/35',
          'group-hover:bg-fg-muted group-focus-visible:bg-fg-muted',
        )}
        style={{ height: busiest === 0 ? 0 : `${String((value / busiest) * 100)}%` }}
      />
    </button>
  );
}

/** One column per value. */
export function ColumnChart({ label, values, name, tick }: ColumnChartProps) {
  const [shown, setShown] = useState<number | null>(null);
  const busiest = Math.max(0, ...values);
  const focus = shown ?? values.indexOf(busiest);
  const readout = busiest === 0 ? '' : statsCopy.barReadout(name(focus), values[focus] ?? 0);
  return (
    <figure className="flex flex-col gap-2" aria-label={label}>
      <figcaption className="h-4 text-xs text-fg-muted tabular-nums" aria-live="polite">
        {readout}
      </figcaption>
      <div
        className="flex items-end gap-0.5 border-b border-border"
        style={{ height: HEIGHT_PX }}
        onPointerLeave={() => {
          setShown(null);
        }}
      >
        {values.map((value, index) => (
          <Column
            key={index}
            value={value}
            busiest={busiest}
            label={statsCopy.barReadout(name(index), value)}
            onShow={() => {
              setShown(index);
            }}
          />
        ))}
      </div>
      <div className="flex gap-0.5" aria-hidden>
        {values.map((_, index) => (
          <span key={index} className="min-w-0 flex-1 text-center text-xs text-fg-subtle">
            {tick(index)}
          </span>
        ))}
      </div>
    </figure>
  );
}

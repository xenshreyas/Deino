/**
 * The people met most: a ranked list, each name linking to its person page,
 * with a thin bar for its share of the busiest and the count at the end.
 */
import { Link } from '@tanstack/react-router';

import { statsCopy } from '../copy';
import type { TopPerson } from '../types';
import { RestingLine } from './stat-section';

/** Props of {@link TopPeople}. */
export interface TopPeopleProps {
  people: readonly TopPerson[];
}

/** Up to eight people, most meetings first. */
export function TopPeople({ people }: TopPeopleProps) {
  const most = people[0]?.meetings ?? 0;
  if (people.length === 0) return <RestingLine>{statsCopy.noPeople}</RestingLine>;
  return (
    <ol className="flex flex-col">
      {people.map((person) => (
        <li key={person.id}>
          <Link
            to="/people/$personId"
            params={{ personId: person.id }}
            className="-mx-2 grid h-8 grid-cols-[minmax(0,10rem)_1fr_2rem] items-center gap-3 rounded-sm px-2 text-sm text-fg transition-colors duration-120 hover:bg-bg-muted"
          >
            <span className="truncate">{person.name}</span>
            <span className="h-1.5 rounded-full bg-bg-muted" aria-hidden>
              <span
                className="block h-full rounded-full bg-fg-subtle/60"
                style={{ width: `${String((person.meetings / Math.max(1, most)) * 100)}%` }}
              />
            </span>
            <span className="text-right text-fg-muted tabular-nums">{person.meetings}</span>
          </Link>
        </li>
      ))}
    </ol>
  );
}

/**
 * The grouped list (ui.md §3 List row): sticky group headers, then rows.
 * Rows are keyed on their status too, so a stored status change resets a
 * row's lingering tick.
 */
import { groupItems } from '../groups';
import type { ActionItemId, ActionItemView, PersonOption } from '../types';
import { ItemRow, type RowHandlers } from './item-row';

/** Props of {@link ItemList}. */
export interface ItemListProps {
  items: readonly ActionItemView[];
  people: readonly PersonOption[];
  now: Date;
  handlers: RowHandlers;
  selected?: ReadonlySet<ActionItemId>;
  showMeeting?: boolean;
}

/** The items under their due-date headers. */
export function ItemList({
  items,
  people,
  now,
  handlers,
  selected,
  showMeeting = true,
}: ItemListProps) {
  return (
    <div className="flex flex-col">
      {groupItems(items, now).map((group) => (
        <section key={group.id} aria-label={group.label}>
          <h2 className="sticky top-0 z-(--z-sticky) flex h-8 items-center bg-bg px-3 text-xs font-semibold text-fg-muted">
            {group.label}
            <span className="ml-2 font-normal text-fg-subtle tabular-nums">
              {group.items.length}
            </span>
          </h2>
          <ul className="flex flex-col">
            {group.items.map((item) => (
              <ItemRow
                key={`${item.id}:${item.status}`}
                item={item}
                people={people}
                now={now}
                handlers={handlers}
                selected={selected?.has(item.id) ?? false}
                showMeeting={showMeeting}
              />
            ))}
          </ul>
        </section>
      ))}
    </div>
  );
}

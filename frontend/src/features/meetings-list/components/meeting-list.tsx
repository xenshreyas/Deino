/**
 * The list body: meetings under one sticky header per day, and a sentinel
 * that loads the next page as it scrolls into view (cursor pagination, so
 * thousands of meetings never load at once).
 */
import { useRef } from 'react';

import { useMountEffect } from '@/hooks/useMountEffect';

import { copy } from '../copy';
import { groupByDay } from '../day-groups';
import type { ListedMeeting, MeetingId } from '../types';
import { MeetingListRow } from './meeting-list-row';

/** Props of {@link MeetingList}. */
export interface MeetingListProps {
  meetings: readonly ListedMeeting[];
  selected: ReadonlySet<MeetingId>;
  onSelect: (id: MeetingId, range: boolean) => void;
  /** Loads the next page; `null` when every page is loaded. */
  onLoadMore: (() => void) | null;
}

/** Calls `onVisible` once when it scrolls near the viewport. Remount it per page. */
function LoadMore({ onVisible }: { onVisible: () => void }) {
  const sentinel = useRef<HTMLDivElement>(null);
  useMountEffect(() => {
    const element = sentinel.current;
    if (element === null) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          observer.disconnect();
          onVisible();
        }
      },
      { rootMargin: '400px' },
    );
    observer.observe(element);
    return () => {
      observer.disconnect();
    };
  });
  return (
    <div ref={sentinel} role="status" className="h-10 px-3 py-3 text-xs text-fg-subtle">
      {copy.loadMore}
    </div>
  );
}

/** The grouped rows. */
export function MeetingList({ meetings, selected, onSelect, onLoadMore }: MeetingListProps) {
  const selecting = selected.size > 0;
  return (
    <>
      {groupByDay(meetings, new Date()).map((group) => (
        <section key={group.key} aria-label={group.label}>
          <h2 className="sticky top-0 z-(--z-sticky) flex h-8 items-center bg-bg px-3 text-xs font-semibold text-fg-muted">
            {group.label}
          </h2>
          <ul className="flex flex-col">
            {group.meetings.map((meeting) => (
              <MeetingListRow
                key={meeting.id}
                meeting={meeting}
                selected={selected.has(meeting.id)}
                selecting={selecting}
                onSelect={(range) => {
                  onSelect(meeting.id, range);
                }}
              />
            ))}
          </ul>
        </section>
      ))}
      {onLoadMore === null ? null : <LoadMore key={meetings.length} onVisible={onLoadMore} />}
    </>
  );
}

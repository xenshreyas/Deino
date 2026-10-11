/**
 * Navigation to where a search result leads, through TanStack Router.
 */
import { useNavigate } from '@tanstack/react-router';

import type { HitTarget } from '../hit-target';
import type { FilterChip } from '../types';

/** Every place search can take the user: a hit's target or the results page. */
export type SearchDestination =
  Exclude<HitTarget, { kind: 'filter' }> | { kind: 'results'; text: string; filters: FilterChip[] };

/** A function that navigates to a search destination. */
export function useOpenHit(): (destination: SearchDestination) => void {
  const navigate = useNavigate();
  return (destination) => {
    switch (destination.kind) {
      case 'meeting':
        void navigate({
          to: '/meetings/$meetingId',
          params: { meetingId: destination.meetingId },
          search: destination.atMs === null ? {} : { tab: 'transcript', t: destination.atMs },
        });
        return;
      case 'folder':
        void navigate({ to: '/folders/$folderId', params: { folderId: destination.folderId } });
        return;
      case 'action-items':
        void navigate({ to: '/action-items' });
        return;
      case 'results':
        void navigate({
          to: '/search',
          search: { q: destination.text, filters: destination.filters },
        });
    }
  };
}

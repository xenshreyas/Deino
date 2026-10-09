import { createFileRoute, redirect } from '@tanstack/react-router';

import { legacyMeetingSearchSchema } from '@/features/meeting-details/search';

/**
 * `/meeting-details?id=`: the Next.js-era meeting URL, kept so old links keep
 * working. Redirects to `/meetings/$meetingId`, or home without an id.
 */
export const Route = createFileRoute('/meeting-details')({
  validateSearch: legacyMeetingSearchSchema,
  beforeLoad: ({ search }) => {
    if (search.id === undefined) throw redirect({ to: '/', replace: true });
    throw redirect({
      to: '/meetings/$meetingId',
      params: { meetingId: search.id },
      search: { source: search.source },
      replace: true,
    });
  },
});

import { createFileRoute } from '@tanstack/react-router';

import { MeetingScreen } from '@/features/meeting';
import { meetingSearchSchema } from '@/features/meeting/search';
import { asMeetingId, meetingQuery } from '@/features/meetings';

/** `/meetings/$meetingId?tab=`: one meeting's notes, transcript and panels. */
export const Route = createFileRoute('/meetings/$meetingId')({
  validateSearch: meetingSearchSchema,
  // Warm the header's query; the page shows a missing meeting itself.
  loader: async ({ context, params }) => {
    await context.queryClient
      .query(meetingQuery(asMeetingId(params.meetingId)))
      .catch(() => undefined);
  },
  component: MeetingRoute,
});

function MeetingRoute() {
  const { meetingId } = Route.useParams();
  const { source, tab = 'notes' } = Route.useSearch();
  const navigate = Route.useNavigate();
  return (
    <MeetingScreen
      meetingId={meetingId}
      source={source}
      tab={tab}
      onTabChange={(next) =>
        void navigate({ search: (previous) => ({ ...previous, tab: next }), replace: true })
      }
    />
  );
}

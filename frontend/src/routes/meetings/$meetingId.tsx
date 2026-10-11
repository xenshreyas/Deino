import { createFileRoute } from '@tanstack/react-router';

import { MeetingScreen } from '@/features/meeting';
import { meetingSearchSchema } from '@/features/meeting/search';
import { asMeetingId, meetingQuery } from '@/features/meetings';
import type { TranscriptMoment } from '@/features/transcript';

/** `/meetings/$meetingId?tab=&t=&segment=`: one meeting's notes, transcript and panels. */
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

/** The moment a link names: a line (`segment`) wins over a time (`t`). */
function momentOf(
  t: number | undefined,
  segment: string | undefined,
): TranscriptMoment | undefined {
  if (segment !== undefined) return { kind: 'line', segmentId: segment };
  return t === undefined ? undefined : { kind: 'time', at: t };
}

function MeetingRoute() {
  const { meetingId } = Route.useParams();
  const { source, tab, t, segment } = Route.useSearch();
  const navigate = Route.useNavigate();
  const moment = momentOf(t, segment);
  return (
    <MeetingScreen
      meetingId={meetingId}
      source={source}
      tab={tab ?? (moment === undefined ? 'notes' : 'transcript')}
      onTabChange={(next) =>
        void navigate({ search: (previous) => ({ ...previous, tab: next }), replace: true })
      }
      moment={moment}
      onMomentShown={() =>
        void navigate({
          search: (previous) => ({ ...previous, t: undefined, segment: undefined }),
          replace: true,
        })
      }
    />
  );
}

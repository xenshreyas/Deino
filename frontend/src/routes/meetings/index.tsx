import { createFileRoute } from '@tanstack/react-router';

import { meetingListQuery, MeetingsScreen } from '@/features/meetings';
import { meetingsSearchSchema } from '@/features/meetings/search';
import { requestRecordingStart, SHORTCUTS } from '@/features/shell';
import { asTagId } from '@/features/tags';

/** `/meetings?tags=`: every meeting, newest first, optionally filtered by tag. */
export const Route = createFileRoute('/meetings/')({
  validateSearch: meetingsSearchSchema,
  // Warm the cache before the screen renders; the screen shows any error itself.
  loader: async ({ context }) => {
    await context.queryClient.query(meetingListQuery()).catch(() => undefined);
  },
  component: MeetingsRoute,
});

const SCREEN_SHORTCUTS = { newNote: SHORTCUTS.newNote.display, record: SHORTCUTS.record.display };

function MeetingsRoute() {
  const navigate = Route.useNavigate();
  const { tags = [] } = Route.useSearch();
  return (
    <MeetingsScreen
      shortcuts={SCREEN_SHORTCUTS}
      tagFilter={tags.map(asTagId)}
      onTagFilterChange={(next) =>
        void navigate({ search: { tags: next.length === 0 ? undefined : next }, replace: true })
      }
      onStartRecording={() => {
        requestRecordingStart('/meetings', () => void navigate({ to: '/' }));
      }}
    />
  );
}

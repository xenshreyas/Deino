import { createFileRoute } from '@tanstack/react-router';

import { meetingListQuery, MeetingsScreen } from '@/features/meetings';
import { requestRecordingStart, SHORTCUTS } from '@/features/shell';

/** `/meetings`: every meeting, newest first. */
export const Route = createFileRoute('/meetings/')({
  // Warm the cache before the screen renders; the screen shows any error itself.
  loader: async ({ context }) => {
    await context.queryClient.query(meetingListQuery()).catch(() => undefined);
  },
  component: MeetingsRoute,
});

const SCREEN_SHORTCUTS = { newNote: SHORTCUTS.newNote.display, record: SHORTCUTS.record.display };

function MeetingsRoute() {
  const navigate = Route.useNavigate();
  return (
    <MeetingsScreen
      shortcuts={SCREEN_SHORTCUTS}
      onStartRecording={() => {
        requestRecordingStart('/meetings', () => void navigate({ to: '/' }));
      }}
    />
  );
}

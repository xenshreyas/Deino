import { createFileRoute } from '@tanstack/react-router';

import { MeetingsListScreen } from '@/features/meetings-list';
import { meetingsSearchSchema } from '@/features/meetings-list/search';
import { requestRecordingStart, SHORTCUTS } from '@/features/shell';
import { asTagId } from '@/features/tags';

/** `/meetings?tags=`: the Meetings page (ORG-07), optionally opened on a tag filter. */
export const Route = createFileRoute('/meetings/')({
  validateSearch: meetingsSearchSchema,
  component: MeetingsRoute,
});

const SCREEN_SHORTCUTS = { newNote: SHORTCUTS.newNote.display, record: SHORTCUTS.record.display };

function MeetingsRoute() {
  const navigate = Route.useNavigate();
  const { tags = [] } = Route.useSearch();
  return (
    <MeetingsListScreen
      shortcuts={SCREEN_SHORTCUTS}
      tags={tags.map(asTagId)}
      onStartRecording={() => {
        void requestRecordingStart(() => void navigate({ to: '/' }));
      }}
    />
  );
}

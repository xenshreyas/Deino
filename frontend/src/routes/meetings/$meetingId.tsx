import { createFileRoute } from '@tanstack/react-router';

import { MeetingDetailsScreen } from '@/features/meeting-details';
import { meetingSearchSchema } from '@/features/meeting-details/search';

/** `/meetings/$meetingId`: one meeting's transcript and summary. */
export const Route = createFileRoute('/meetings/$meetingId')({
  validateSearch: meetingSearchSchema,
  component: MeetingRoute,
});

function MeetingRoute() {
  const { meetingId } = Route.useParams();
  const { source } = Route.useSearch();
  return <MeetingDetailsScreen meetingId={meetingId} source={source} />;
}

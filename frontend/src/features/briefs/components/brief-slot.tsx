/**
 * The brief containers: {@link EventBrief} for an upcoming event (ORG-02's
 * event note) and {@link MeetingBrief} for a meeting note linked to one.
 * They render nothing when there is no brief, mark it read once shown (so
 * it opens collapsed next time), store the thumbs down and follow
 * citations.
 */
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { toast } from 'sonner';

import { useMountEffect } from '@/hooks/useMountEffect';
import type { CalendarEventId } from '@/lib/ipc/generated/calendar';
import type { MeetingId } from '@/lib/ipc/generated/meetings';

import { briefsCopy } from '../copy';
import { useBriefFeedback, useEventBrief, useMarkBriefRead, useMeetingBrief } from '../queries';
import type { Brief, BriefCitation } from '../types';
import { BriefCard } from './brief-card';

/** Navigates to what a citation names. */
function useOpenCitation(): (citation: BriefCitation) => void {
  const navigate = useNavigate();
  return (citation) => {
    switch (citation.kind) {
      case 'meeting':
        void navigate({ to: '/meetings/$meetingId', params: { meetingId: citation.meetingId } });
        return;
      case 'moment':
        void navigate({
          to: '/meetings/$meetingId',
          params: { meetingId: citation.meetingId },
          search: { segment: citation.segmentId },
        });
        return;
      case 'person':
        void navigate({ to: '/people/$personId', params: { personId: citation.personId } });
        return;
      case 'company':
        void navigate({ to: '/companies/$companyId', params: { companyId: citation.companyId } });
        return;
      case 'event':
      case 'action_item':
    }
  };
}

/** A loaded brief: marks it read on first show, keeping it open meanwhile. */
function LoadedBrief({ brief }: { brief: Brief }) {
  const [unread] = useState(brief.readAt === null);
  const markRead = useMarkBriefRead();
  const feedback = useBriefFeedback();
  const openCitation = useOpenCitation();
  useMountEffect(() => {
    if (unread) markRead.mutate(brief.eventId);
  });
  return (
    <BriefCard
      brief={brief}
      defaultExpanded={unread}
      onOpenCitation={openCitation}
      onNotUseful={() => {
        feedback.mutate(
          { event: brief.eventId, feedback: 'not_useful' },
          {
            onSuccess: () => toast(briefsCopy.thanks),
            onError: (error) =>
              toast.error(briefsCopy.feedbackFailed, { description: error.message }),
          },
        );
      }}
    />
  );
}

/** The brief of an upcoming event, if there is one. */
export function EventBrief({ eventId }: { eventId: CalendarEventId }) {
  const brief = useEventBrief(eventId);
  return brief.data ? <LoadedBrief key={brief.data.eventId} brief={brief.data} /> : null;
}

/** The brief of the event a meeting note is linked to, if there is one. */
export function MeetingBrief({ meetingId }: { meetingId: MeetingId }) {
  const brief = useMeetingBrief(meetingId);
  if (!brief.data) return null;
  return (
    <div className="px-6 pt-3">
      <LoadedBrief key={brief.data.eventId} brief={brief.data} />
    </div>
  );
}

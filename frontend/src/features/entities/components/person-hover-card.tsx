/**
 * The hover card of a person (Granola's attendee cards, ui.md §2.8): who
 * they are, their last three meetings with the user, and quick actions
 * (open their page, email them). It wraps any trigger: an attendee chip, an
 * @mention, a row. The page is read only once the card opens.
 */
import { useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { type ReactNode, useState } from 'react';

import { buttonVariants } from '@/ui/button';
import {
  HOVER_CLOSE_DELAY,
  HOVER_OPEN_DELAY,
  HoverCard,
  HoverCardContent,
  HoverCardTrigger,
} from '@/ui/hover-card';
import { ArrowUpRight, ICON_PROPS, Mail, NotebookText } from '@/ui/icons';
import { SkeletonText } from '@/ui/skeleton';

import { entitiesCopy } from '../copy';
import { relativeDay } from '../format';
import { personPageQuery } from '../queries';
import type { PersonId, PersonPage } from '../types';
import { EntityAvatar } from './entity-avatar';

/** Meetings the card lists. */
const RECENT = 3;

/** Their last few meetings with the user, each opening the meeting. */
function RecentMeetings({ meetings }: { meetings: PersonPage['meetings'] }) {
  const now = new Date();
  if (meetings.length === 0) {
    return <p className="text-xs text-fg-subtle">{entitiesCopy.noRecent}</p>;
  }
  return meetings.slice(0, RECENT).map((meeting) => (
    <Link
      key={meeting.id}
      to="/meetings/$meetingId"
      params={{ meetingId: meeting.id }}
      className="-mx-1.5 flex h-7 items-center gap-2 rounded-sm px-1.5 text-xs hover:bg-bg-muted"
    >
      <NotebookText {...ICON_PROPS} size={14} className="shrink-0 text-fg-muted" />
      <span className="min-w-0 flex-1 truncate text-fg">{meeting.title}</span>
      <span className="shrink-0 text-fg-subtle tabular-nums">{relativeDay(meeting.at, now)}</span>
    </Link>
  ));
}

/** The card's body once the page is read. */
function CardBody({ page }: { page: PersonPage }) {
  const { person, company } = page;
  const detail = [person.title, company?.name].filter(Boolean).join(' · ');
  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center gap-3">
        <EntityAvatar
          id={person.id}
          name={person.name}
          kind="person"
          logoPath={company?.logoPath}
          size={32}
        />
        <div className="min-w-0">
          <p className="truncate text-sm font-semibold text-fg">{person.name}</p>
          <p className="truncate text-xs text-fg-muted">{detail === '' ? person.email : detail}</p>
        </div>
      </div>
      <div className="flex flex-col">
        <p className="pb-1 text-xs font-semibold text-fg-muted">{entitiesCopy.recentMeetings}</p>
        <RecentMeetings meetings={page.meetings} />
      </div>
      <CardActions page={page} />
    </div>
  );
}

/** Open the page; email them. */
function CardActions({ page }: { page: PersonPage }) {
  const { person } = page;
  return (
    <div className="flex items-center gap-1.5 border-t border-border pt-2.5">
      <Link
        to="/people/$personId"
        params={{ personId: person.id }}
        className={buttonVariants({ variant: 'secondary', size: 'sm' })}
      >
        <ArrowUpRight {...ICON_PROPS} />
        {entitiesCopy.openPage}
      </Link>
      {person.email === null ? null : (
        <a
          href={`mailto:${person.email}`}
          className={buttonVariants({ variant: 'ghost', size: 'sm' })}
        >
          <Mail {...ICON_PROPS} />
          {entitiesCopy.email}
        </a>
      )}
    </div>
  );
}

/** A person card's content: their page once loaded, a skeleton until then. */
export function PersonCardContent({ personId }: { personId: PersonId }) {
  const page = useQuery(personPageQuery(personId));
  if (page.data === undefined) return <SkeletonText lines={3} />;
  return <CardBody page={page.data} />;
}

/** Props of {@link PersonHoverCard}. */
export interface PersonHoverCardProps {
  personId: PersonId;
  /** The trigger; it must accept a ref (a button, a link). */
  children: ReactNode;
  /** Where the card opens; `right` beside a list, so it covers no row. */
  side?: 'bottom' | 'right';
}

/** `children` with the person's card on hover and focus. */
export function PersonHoverCard({ personId, children, side = 'bottom' }: PersonHoverCardProps) {
  const [open, setOpen] = useState(false);
  return (
    <HoverCard
      open={open}
      onOpenChange={setOpen}
      openDelay={HOVER_OPEN_DELAY}
      closeDelay={HOVER_CLOSE_DELAY}
    >
      <HoverCardTrigger asChild>{children}</HoverCardTrigger>
      <HoverCardContent side={side}>
        {open ? <PersonCardContent personId={personId} /> : null}
      </HoverCardContent>
    </HoverCard>
  );
}

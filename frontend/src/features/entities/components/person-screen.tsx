/**
 * A person's page (`/people/$personId`, ORG-04, ui.md §4.5): who they are,
 * the AI briefing, what is coming up and still open with them, every
 * meeting together, the user's notes, their voice, and the resting slot
 * for chat about them. A merged person's link shows their survivor.
 */
import { Link } from '@tanstack/react-router';

import { EmptyState } from '@/ui/empty-state';
import { UserRound } from '@/ui/icons';
import { Page } from '@/ui/page-header';
import { Skeleton, SkeletonText } from '@/ui/skeleton';

import { entitiesCopy } from '../copy';
import { useRefreshPersonSummary, useSetPersonNotes } from '../mutations';
import { usePersonPage } from '../queries';
import { ENTITY_SLOTS } from '../slots';
import type { PersonId, PersonPage } from '../types';
import { EntityAvatar } from './entity-avatar';
import { EntityHeader, META_LINK } from './entity-header';
import { EntityNotes } from './entity-notes';
import { ActionItemRows, MeetingRows, Section, UpcomingRows } from './entity-sections';
import { EntitySummary } from './entity-summary';
import { PersonActions } from './person-actions';

/** Title, company (a link) and address, whichever are known. */
function metaOf({ person, company }: PersonPage) {
  const meta = [];
  if (person.title !== null) meta.push(<span className="truncate">{person.title}</span>);
  if (company !== null) {
    meta.push(
      <Link to="/companies/$companyId" params={{ companyId: company.id }} className={META_LINK}>
        {company.name}
      </Link>,
    );
  }
  if (person.email !== null) meta.push(<span className="truncate">{person.email}</span>);
  return meta;
}

/** Header, the ask slot and the briefing. */
function PersonTop({ page }: { page: PersonPage }) {
  const { person } = page;
  const refresh = useRefreshPersonSummary();
  const Ask = ENTITY_SLOTS.ask;
  const avatar = (
    <EntityAvatar
      id={person.id}
      name={person.name}
      kind="person"
      logoPath={page.company?.logoPath}
      size={48}
    />
  );
  return (
    <>
      <EntityHeader
        avatar={avatar}
        name={person.name}
        meta={metaOf(page)}
        actions={<PersonActions person={person} />}
      />
      {Ask === null ? null : <Ask scope={{ kind: 'person', id: person.id }} name={person.name} />}
      <EntitySummary
        summary={person.summaryMd}
        updatedAt={person.summaryUpdatedAt}
        refresh={() => refresh.mutateAsync(person.id)}
        refreshing={refresh.isPending}
      />
    </>
  );
}

/** What is coming up and open, the meetings, the notes and the voice. */
function PersonSections({ page }: { page: PersonPage }) {
  const { person } = page;
  const notes = useSetPersonNotes();
  const now = new Date();
  const Voice = ENTITY_SLOTS.voice;
  return (
    <>
      {page.upcoming.length === 0 ? null : (
        <Section id="entity-upcoming" title={entitiesCopy.upcoming}>
          <UpcomingRows events={page.upcoming} />
        </Section>
      )}
      <Section id="entity-items" title={entitiesCopy.actionItems}>
        <ActionItemRows items={page.actionItems} now={now} />
      </Section>
      <Section id="entity-meetings" title={entitiesCopy.meetings}>
        <MeetingRows meetings={page.meetings} now={now} />
      </Section>
      <EntityNotes
        key={person.id}
        initial={person.notesMd}
        save={(text) => notes.mutateAsync({ id: person.id, notes: text })}
      />
      <Voice personId={person.id} />
    </>
  );
}

/** The header's and summary's shape while the page loads. */
export function PageSkeleton() {
  return (
    <div aria-busy className="flex flex-col gap-7">
      <div className="flex items-center gap-4">
        <Skeleton className="size-12 rounded-full" />
        <div className="flex flex-1 flex-col gap-2">
          <Skeleton className="h-5 w-2/5" />
          <Skeleton className="h-2.5 w-1/3" />
        </div>
      </div>
      <SkeletonText lines={4} />
    </div>
  );
}

/** The person page. */
export function PersonScreen({ personId }: { personId: PersonId }) {
  const page = usePersonPage(personId);
  if (page.isError) return <EmptyState icon={UserRound} title={entitiesCopy.notFound} />;
  return (
    <Page width="document">
      {page.data === undefined ? (
        <PageSkeleton />
      ) : (
        <div className="flex flex-col gap-7">
          <PersonTop page={page.data} />
          <PersonSections page={page.data} />
        </div>
      )}
    </Page>
  );
}

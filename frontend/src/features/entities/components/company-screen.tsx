/**
 * A company's page (`/companies/$companyId`, ORG-04): its logo and website,
 * the AI briefing, its people, what is coming up and still open with them,
 * every meeting with any of them, the user's notes, and the resting slot
 * for chat about it. The logo is fetched once on the first visit, when the
 * user allows company logos.
 */
import { Link } from '@tanstack/react-router';

import { useMountEffect } from '@/hooks/useMountEffect';
import { EmptyState } from '@/ui/empty-state';
import { Building2 } from '@/ui/icons';
import { Page } from '@/ui/page-header';

import { entitiesCopy } from '../copy';
import { relativeDay } from '../format';
import { useFetchLogo, useRefreshCompanySummary, useSetCompanyNotes } from '../mutations';
import { useCompanyPage } from '../queries';
import { ENTITY_SLOTS } from '../slots';
import type { Company, CompanyId, CompanyPage, EntityActionItem, PersonRow } from '../types';
import { CompanyActions } from './company-actions';
import { EntityAvatar } from './entity-avatar';
import { EntityHeader, META_LINK } from './entity-header';
import { EntityNotes } from './entity-notes';
import { ActionItemRows, MeetingRows, Quiet, Section, UpcomingRows } from './entity-sections';
import { EntitySummary } from './entity-summary';
import { PageSkeleton } from './person-screen';

/** The website (a link) and how many people, whichever are known. */
function metaOf({ company, people }: CompanyPage) {
  const meta = [];
  if (company.domain !== null) {
    meta.push(
      <a href={`https://${company.domain}`} target="_blank" rel="noreferrer" className={META_LINK}>
        {company.domain}
      </a>,
    );
  }
  if (people.length > 0) meta.push(<span>{entitiesCopy.peopleCount(people.length)}</span>);
  return meta;
}

/** The company's people, most recently met first. */
function PeopleRows({ people, now }: { people: readonly PersonRow[]; now: Date }) {
  if (people.length === 0) return <Quiet>{entitiesCopy.noPeopleAt}</Quiet>;
  return (
    <ul className="flex flex-col">
      {people.map((person) => (
        <li key={person.id}>
          <Link
            to="/people/$personId"
            params={{ personId: person.id }}
            className="flex h-10 items-center gap-3 rounded-md px-3 text-sm transition-colors duration-(--dur-fast) hover:bg-bg-muted focus-visible:bg-bg-muted"
          >
            <EntityAvatar id={person.id} name={person.name} kind="person" size={20} />
            <span className="truncate text-fg">{person.name}</span>
            <span className="min-w-0 flex-1 truncate text-xs text-fg-muted">{person.title}</span>
            <span className="shrink-0 text-xs text-fg-muted tabular-nums">
              {person.lastMetAt === null
                ? entitiesCopy.neverMet
                : relativeDay(person.lastMetAt, now)}
            </span>
          </Link>
        </li>
      ))}
    </ul>
  );
}

/** Asks for the logo once, quietly: a refusal (logos off) changes nothing. */
function useLogoOnFirstVisit(company: Company) {
  const fetchLogo = useFetchLogo();
  useMountEffect(() => {
    if (company.domain === null || company.logoCheckedAt !== null) return;
    fetchLogo.mutate({ id: company.id, refresh: false });
  });
}

/** Header, the ask slot and the briefing. */
function CompanyTop({ page }: { page: CompanyPage }) {
  const { company } = page;
  const refresh = useRefreshCompanySummary();
  useLogoOnFirstVisit(company);
  const Ask = ENTITY_SLOTS.ask;
  const avatar = (
    <EntityAvatar
      id={company.id}
      name={company.name}
      kind="company"
      logoPath={company.logoPath}
      size={48}
    />
  );
  return (
    <>
      <EntityHeader
        avatar={avatar}
        name={company.name}
        meta={metaOf(page)}
        actions={<CompanyActions company={company} />}
      />
      <Ask scope={{ kind: 'company', id: company.id }} name={company.name} />
      <EntitySummary
        summary={company.summaryMd}
        updatedAt={company.summaryUpdatedAt}
        refresh={() => refresh.mutateAsync(company.id)}
        refreshing={refresh.isPending}
      />
    </>
  );
}

/** Its people, what is coming up and open, the meetings and the notes. */
function CompanySections({ page }: { page: CompanyPage }) {
  const { company, people } = page;
  const notes = useSetCompanyNotes();
  const now = new Date();
  const names = new Map(people.map((person) => [person.id, person.name]));
  const owner = (item: EntityActionItem) =>
    item.assigneePersonId === null ? null : (names.get(item.assigneePersonId) ?? null);
  return (
    <>
      <Section id="entity-people" title={entitiesCopy.peopleAt}>
        <PeopleRows people={people} now={now} />
      </Section>
      {page.upcoming.length === 0 ? null : (
        <Section id="entity-upcoming" title={entitiesCopy.upcoming}>
          <UpcomingRows events={page.upcoming} />
        </Section>
      )}
      <Section id="entity-items" title={entitiesCopy.actionItems}>
        <ActionItemRows items={page.actionItems} now={now} owner={owner} />
      </Section>
      <Section id="entity-meetings" title={entitiesCopy.meetings}>
        <MeetingRows meetings={page.meetings} now={now} />
      </Section>
      <EntityNotes
        key={company.id}
        initial={company.notesMd}
        save={(text) => notes.mutateAsync({ id: company.id, notes: text })}
      />
    </>
  );
}

/** The company page. */
export function CompanyScreen({ companyId }: { companyId: CompanyId }) {
  const page = useCompanyPage(companyId);
  if (page.isError) return <EmptyState icon={Building2} title={entitiesCopy.notFound} />;
  return (
    <Page width="document">
      {page.data === undefined ? (
        <PageSkeleton />
      ) : (
        <div className="flex flex-col gap-7">
          <CompanyTop page={page.data} />
          <CompanySections page={page.data} />
        </div>
      )}
    </Page>
  );
}

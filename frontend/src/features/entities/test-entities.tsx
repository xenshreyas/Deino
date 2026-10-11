/**
 * Test helpers for the people and companies components: rows and pages
 * shaped like the generated types, and a renderer with a query client, a
 * memory router (every entity and meeting route) and tooltips.
 */
import { QueryClientProvider } from '@tanstack/react-query';
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from '@tanstack/react-router';
import { render } from '@testing-library/react';
import type { ReactNode } from 'react';

import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import type {
  Company,
  CompanyId,
  CompanyPage,
  MeetingId,
  Person,
  PersonId,
  PersonPage,
  PersonRow,
} from './types';

export const NOW = new Date().toISOString();
/** `days` before now, as an ISO timestamp. */
export function daysAgo(days: number): string {
  return new Date(Date.now() - days * 86_400_000).toISOString();
}

export const MAYA = 'person-maya' as PersonId;
export const OWEN = 'person-owen' as PersonId;
export const NOOR = 'person-noor' as PersonId;
export const DEINO = 'company-deino' as CompanyId;

/** A person row; `lastMetAt` days ago, or never. */
export function row(id: PersonId, name: string, days: number | null): PersonRow {
  return {
    id,
    name,
    email: `${name.split(' ')[0]?.toLowerCase() ?? 'x'}@deino.dev`,
    title: null,
    companyId: null,
    companyName: null,
    companyLogoPath: null,
    lastMetAt: days === null ? null : daysAgo(days),
    meetingCount: days === null ? 0 : 1,
  };
}

/** A stored person. */
export function person(id: PersonId, name: string, patch: Partial<Person> = {}): Person {
  return {
    id,
    name,
    email: null,
    companyId: null,
    title: null,
    avatarPath: null,
    notesMd: '',
    summaryMd: null,
    summaryUpdatedAt: null,
    isMe: false,
    mergedInto: null,
    createdAt: NOW,
    updatedAt: NOW,
    ...patch,
  };
}

/** A stored company. */
export function company(id: CompanyId, name: string, patch: Partial<Company> = {}): Company {
  return {
    id,
    name,
    domain: null,
    logoPath: null,
    notesMd: '',
    summaryMd: null,
    summaryUpdatedAt: null,
    logoCheckedAt: null,
    mergedInto: null,
    createdAt: NOW,
    updatedAt: NOW,
    ...patch,
  };
}

/** A person page with one meeting and one open item. */
export function personPage(subject: Person): PersonPage {
  return {
    person: subject,
    company: null,
    meetings: [
      { id: 'meeting-1' as MeetingId, title: '1:1 Maya', at: daysAgo(2), durationS: 1140 },
    ],
    actionItems: [
      {
        id: 'item-1' as PersonPage['actionItems'][number]['id'],
        title: 'Split the webhook queue',
        dueAt: daysAgo(1),
        assigneePersonId: subject.id,
        meetingId: 'meeting-1' as MeetingId,
      },
    ],
    upcoming: [],
  };
}

/** A company page listing `people`. */
export function companyPage(subject: Company, people: PersonRow[]): CompanyPage {
  return { company: subject, people, meetings: [], actionItems: [], upcoming: [] };
}

const PATHS = [
  '/people',
  '/people/$personId',
  '/companies',
  '/companies/$companyId',
  '/meetings/$meetingId',
  '/settings',
] as const;

/** Renders `Screen` at `url` with every route the entity pages link to. */
export function renderAt(url: string, Screen: () => ReactNode) {
  const root = createRootRoute({ component: Screen });
  const routes = PATHS.map((path) => createRoute({ getParentRoute: () => root, path }));
  const router = createRouter({
    routeTree: root.addChildren(routes),
    history: createMemoryHistory({ initialEntries: [url] }),
  });
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <RouterProvider router={router} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

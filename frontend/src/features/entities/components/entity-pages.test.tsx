import { fireEvent, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type * as CompanyBindings from '@/lib/ipc/generated/companies';
import type * as PeopleBindings from '@/lib/ipc/generated/people';
import type * as SpeakerBindings from '@/lib/ipc/generated/speakers';

import {
  company,
  companyPage,
  DEINO,
  MAYA,
  OWEN,
  person,
  personPage,
  renderAt,
  row,
} from '../test-entities';
import { asCompanyId, asPersonId } from '../types';
import { CompanyScreen } from './company-screen';
import { PersonScreen } from './person-screen';

const people = vi.hoisted(
  () =>
    ({
      search: vi.fn<typeof PeopleBindings.commands.search>(),
      list: vi.fn<typeof PeopleBindings.commands.list>(),
      page: vi.fn<typeof PeopleBindings.commands.page>(),
      create: vi.fn<typeof PeopleBindings.commands.create>(),
      update: vi.fn<typeof PeopleBindings.commands.update>(),
      setNotes: vi.fn<typeof PeopleBindings.commands.setNotes>(),
      merge: vi.fn<typeof PeopleBindings.commands.merge>(),
      fromMention: vi.fn<typeof PeopleBindings.commands.fromMention>(),
      refreshSummary: vi.fn<typeof PeopleBindings.commands.refreshSummary>(),
      attendees: vi.fn<typeof PeopleBindings.commands.attendees>(),
      addAttendee: vi.fn<typeof PeopleBindings.commands.addAttendee>(),
      removeAttendee: vi.fn<typeof PeopleBindings.commands.removeAttendee>(),
    }) satisfies typeof PeopleBindings.commands,
);
vi.mock('@/lib/ipc/generated/people', () => ({ commands: people }));
const companies = vi.hoisted(
  () =>
    ({
      list: vi.fn<typeof CompanyBindings.commands.list>(),
      page: vi.fn<typeof CompanyBindings.commands.page>(),
      create: vi.fn<typeof CompanyBindings.commands.create>(),
      update: vi.fn<typeof CompanyBindings.commands.update>(),
      setNotes: vi.fn<typeof CompanyBindings.commands.setNotes>(),
      merge: vi.fn<typeof CompanyBindings.commands.merge>(),
      refreshSummary: vi.fn<typeof CompanyBindings.commands.refreshSummary>(),
      fetchLogo: vi.fn<typeof CompanyBindings.commands.fetchLogo>(),
    }) satisfies typeof CompanyBindings.commands,
);
vi.mock('@/lib/ipc/generated/companies', () => ({ commands: companies }));
const speakers = vi.hoisted(() => ({
  voiceProfiles: vi.fn<typeof SpeakerBindings.commands.voiceProfiles>(),
}));
vi.mock('@/lib/ipc/generated/speakers', () => ({ commands: speakers, events: {} }));

const maya = person(MAYA, 'Maya Chen', {
  email: 'maya@deino.dev',
  title: 'Senior Backend Engineer',
  summaryMd: 'Owns the webhook queue.',
  summaryUpdatedAt: new Date().toISOString(),
});

function MayaPage() {
  return <PersonScreen personId={asPersonId(MAYA)} />;
}

function DeinoPage() {
  return <CompanyScreen companyId={asCompanyId(DEINO)} />;
}

describe('person page', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    people.page.mockResolvedValue({ status: 'ok', data: personPage(maya) });
    people.list.mockResolvedValue({
      status: 'ok',
      data: [row(MAYA, 'Maya Chen', 2), row(OWEN, 'Maya C.', 9)],
    });
    speakers.voiceProfiles.mockResolvedValue({ status: 'ok', data: [] });
  });

  test('shows who they are, the AI briefing, open items and meetings', async () => {
    renderAt('/people/person-maya', MayaPage);

    expect(await screen.findByRole('heading', { name: 'Maya Chen' })).toBeDefined();
    expect(screen.getByText('Owns the webhook queue.')).toBeDefined();
    const items = screen.getByRole('region', { name: 'Open action items' });
    expect(within(items).getByText('Split the webhook queue')).toBeDefined();
    const meetings = screen.getByRole('region', { name: 'Meetings' });
    expect(within(meetings).getByRole('link', { name: /1:1 Maya/ })).toBeDefined();
  });

  test('chat about them rests as a disabled WIP bar until Ask Deino ships', async () => {
    renderAt('/people/person-maya', MayaPage);

    const ask = await screen.findByRole('group', { name: 'Ask about Maya Chen' });
    expect(ask.getAttribute('aria-disabled')).toBe('true');
    expect(within(ask).getByText('WIP')).toBeDefined();
  });

  test('merging picks a duplicate and keeps this person', async () => {
    people.merge.mockResolvedValue({ status: 'ok', data: maya });
    renderAt('/people/person-maya', MayaPage);

    fireEvent.pointerDown(await screen.findByRole('button', { name: 'More actions' }), {
      button: 0,
      ctrlKey: false,
    });
    fireEvent.click(await screen.findByRole('menuitem', { name: 'Merge duplicate…' }));
    fireEvent.click(await screen.findByRole('option', { name: /Maya C\./ }));
    fireEvent.click(screen.getByRole('button', { name: 'Merge' }));

    await vi.waitFor(() => {
      expect(people.merge).toHaveBeenCalledWith(MAYA, OWEN);
    });
  });

  test('notes save themselves when the field is left', async () => {
    people.setNotes.mockResolvedValue({ status: 'ok', data: null });
    renderAt('/people/person-maya', MayaPage);

    const notes = await screen.findByRole('textbox', { name: 'Notes' });
    fireEvent.change(notes, { target: { value: 'Prefers async updates.' } });
    fireEvent.blur(notes);

    await vi.waitFor(() => {
      expect(people.setNotes).toHaveBeenCalledWith(MAYA, 'Prefers async updates.');
    });
  });
});

describe('company page', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    const deino = company(DEINO, 'Deino', { domain: 'deino.dev' });
    companies.page.mockResolvedValue({
      status: 'ok',
      data: companyPage(deino, [row(MAYA, 'Maya Chen', 2)]),
    });
    companies.fetchLogo.mockResolvedValue({
      status: 'error',
      error: { code: 'permission_denied', message: 'off' },
    });
  });

  test('lists its people and asks once for the logo on the first visit', async () => {
    renderAt('/companies/company-deino', DeinoPage);

    const people = await screen.findByRole('region', { name: 'People' });
    expect(within(people).getByRole('link', { name: /Maya Chen/ })).toBeDefined();
    expect(screen.getByRole('link', { name: 'deino.dev' })).toBeDefined();
    await vi.waitFor(() => {
      expect(companies.fetchLogo).toHaveBeenCalledWith(DEINO, false);
    });
  });
});

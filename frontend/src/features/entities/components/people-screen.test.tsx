import { fireEvent, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type * as CompanyBindings from '@/lib/ipc/generated/companies';
import type * as PeopleBindings from '@/lib/ipc/generated/people';

import { MAYA, NOOR, OWEN, person, renderAt, row } from '../test-entities';
import { PeopleScreen } from './people-screen';

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

/** The names of the listed rows, top to bottom. */
async function listed(): Promise<string[]> {
  const list = await screen.findByRole('list', { name: 'People' });
  return within(list)
    .getAllByRole('link')
    .map((link) => link.textContent);
}

describe('People page', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    people.list.mockResolvedValue({
      status: 'ok',
      data: [row(OWEN, 'Owen Price', 6), row(MAYA, 'Maya Chen', 1), row(NOOR, 'Noor Haddad', null)],
    });
    companies.list.mockResolvedValue({ status: 'ok', data: [] });
  });

  test('lists people most recently met first, the never-met last', async () => {
    renderAt('/people', PeopleScreen);

    const names = await listed();

    expect(names[0]).toContain('Maya Chen');
    expect(names[1]).toContain('Owen Price');
    expect(names[2]).toContain('Noor Haddad');
    expect(names[2]).toContain('Not met yet');
  });

  test('sorts by name and narrows by search, with a way back from no match', async () => {
    renderAt('/people', PeopleScreen);
    await listed();

    fireEvent.click(screen.getByRole('radio', { name: 'Name' }));
    expect((await listed())[0]).toContain('Maya Chen');
    const search = screen.getByRole('searchbox', { name: 'Search people' });
    fireEvent.change(search, { target: { value: 'pri' } });
    expect(await listed()).toHaveLength(1);
    fireEvent.change(search, { target: { value: 'zed' } });
    fireEvent.click(await screen.findByRole('button', { name: 'Clear search' }));

    expect(await listed()).toHaveLength(3);
  });

  test('adding a person sends the form and opens their page', async () => {
    people.create.mockResolvedValue({ status: 'ok', data: person(NOOR, 'Dana Ortiz') });
    renderAt('/people', PeopleScreen);
    await listed();

    fireEvent.click(screen.getByRole('button', { name: 'Add person' }));
    fireEvent.change(await screen.findByLabelText('Name'), { target: { value: 'Dana Ortiz' } });
    fireEvent.change(screen.getByLabelText('Email'), { target: { value: 'dana@acme.com' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create' }));

    await vi.waitFor(() => {
      expect(people.create).toHaveBeenCalledWith({
        name: 'Dana Ortiz',
        email: 'dana@acme.com',
        title: '',
        companyId: null,
      });
    });
  });

  test('an empty book explains where people come from', async () => {
    people.list.mockResolvedValue({ status: 'ok', data: [] });
    renderAt('/people', PeopleScreen);

    expect(await screen.findByText('No people yet')).toBeDefined();
  });
});

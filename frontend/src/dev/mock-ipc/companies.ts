/**
 * Mock of `@/lib/ipc/generated/companies`: the Companies list and pages,
 * the company form, notes and merges over the mock entity tables. Like the
 * app with default settings, company logos are off (`permission_denied`),
 * and summaries answer `no_model`.
 */
import type * as Generated from '@/lib/ipc/generated/companies';

import { mockDatabase, mockEntities, mockEntityPages } from './database';
import { latency } from './latency';
import { failure, ok } from './mock-database';

type Company = Generated.Company;
type CompanyId = Generated.CompanyId;

let created = 0;

const notFound = (id: string) =>
  failure<never>({ code: 'not_found', message: `company ${id} was not found` });
const invalid = (message: string) => failure<never>({ code: 'invalid', message });

/** The domain Rust reads from a URL, an address or a bare name; `undefined` when it is none. */
function domainOf(input: string | null): string | null | undefined {
  const text = input?.trim() ?? '';
  if (text === '') return null;
  const host = (text.split('@').at(-1) ?? text).replace(/^\w+:\/\//, '').split(/[/?#:]/)[0] ?? '';
  const domain = host.replace(/^www\./, '').toLowerCase();
  return /^[^.\s]+(\.[^.\s]+)+$/.test(domain) ? domain : undefined;
}

/** The form's fields as Rust reads them, or why not. */
function checked(fields: Generated.CompanyFields, self: CompanyId | null) {
  const name = fields.name.trim();
  const domain = domainOf(fields.domain);
  if (name === '') return invalid('a name cannot be empty');
  if (domain === undefined) return invalid('that is not a web domain, such as acme.com');
  const taken = mockEntities.companies.some(
    (company) => company.domain === domain && company.id !== self,
  );
  if (domain !== null && taken) {
    return failure<never>({
      code: 'conflict',
      message: 'another company already has that domain; merge the two instead',
    });
  }
  return ok({ name, domain });
}

/** Replaces `company` and reports the write. */
function save(company: Company): Company {
  mockEntities.companies = mockEntities.companies.map((row) =>
    row.id === company.id ? company : row,
  );
  mockDatabase.touch(['companies']);
  return company;
}

export const commands = {
  async list() {
    await latency();
    return ok(mockEntityPages.companyRows());
  },
  async page(id) {
    await latency();
    const company = mockEntities.company(id);
    const page = company === undefined ? null : mockEntityPages.companyPage(company.id);
    return page === null ? notFound(id) : ok(page);
  },
  async create(fields) {
    await latency();
    const result = checked(fields, null);
    if (result.status === 'error') return result;
    const now = new Date().toISOString();
    created += 1;
    const company: Company = {
      id: `0199d0b0-0000-7000-8000-${String(created).padStart(12, '0')}` as CompanyId,
      ...result.data,
      logoPath: null,
      notesMd: '',
      summaryMd: null,
      summaryUpdatedAt: null,
      logoCheckedAt: null,
      mergedInto: null,
      createdAt: now,
      updatedAt: now,
    };
    mockEntities.companies = [...mockEntities.companies, company];
    mockDatabase.touch(['companies']);
    return ok(company);
  },
  async update(id, fields) {
    await latency();
    const company = mockEntities.company(id);
    const result = checked(fields, id);
    if (company === undefined) return notFound(id);
    if (result.status === 'error') return result;
    return ok(save({ ...company, ...result.data, updatedAt: new Date().toISOString() }));
  },
  async setNotes(id, notes) {
    await latency();
    const company = mockEntities.company(id);
    if (company === undefined) return notFound(id);
    save({ ...company, notesMd: notes });
    return ok(null);
  },
  async merge(survivor, absorbed) {
    await latency();
    if (survivor === absorbed) return invalid('pick two different entries to merge');
    const kept = mockEntities.company(survivor);
    const gone = mockEntities.company(absorbed);
    if (kept === undefined) return notFound(survivor);
    if (gone === undefined) return notFound(absorbed);
    mockEntities.mergeCompanies(kept, gone, new Date().toISOString());
    mockDatabase.touch(['companies', 'people']);
    return ok(kept);
  },
  async refreshSummary(id) {
    await latency();
    return mockEntities.company(id) === undefined ? notFound(id) : ok('no_model' as const);
  },
  async fetchLogo(id) {
    await latency();
    if (mockEntities.company(id) === undefined) return notFound(id);
    return failure({
      code: 'permission_denied',
      message: 'Company logos are off in Settings → Network.',
    });
  },
} satisfies typeof Generated.commands;

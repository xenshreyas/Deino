/**
 * Mock of `@/lib/ipc/generated/templates`: the catalogue's built-ins plus the
 * templates made in this session, and each meeting's choice. Selection
 * follows the Rust order without the profile: the meeting's chosen template,
 * a title rule, then General. It does not repeat every Rust rule; that is
 * the contract tests' job.
 */
import type * as Generated from '@/lib/ipc/generated/templates';

import { mockDatabase } from './database';
import { latency } from './latency';
import { failure, ok } from './mock-database';
import { BUILT_IN_TEMPLATES } from './template-catalog';

type Template = Generated.Template;
type TemplateId = Generated.TemplateId;
type MeetingId = Generated.MeetingId;

let templates: Template[] = [...BUILT_IN_TEMPLATES];
const chosen = new Map<MeetingId, TemplateId>();
let created = 0;

const notFound = (id: string) =>
  failure<never>({ code: 'not_found', message: `template ${id} was not found` });
const readOnly = failure<never>({
  code: 'conflict',
  message: 'built-in templates cannot be changed; duplicate it to edit a copy',
});

/** Saves `content` as a new user template, with the Rust service's name rule. */
function create(content: Generated.TemplateContent) {
  if (content.name.trim() === '')
    return failure<Template>({ code: 'invalid', message: 'name cannot be empty' });
  created += 1;
  const now = new Date().toISOString();
  const template: Template = {
    id: `mock-template-${String(created)}` as TemplateId,
    isBuiltin: false,
    content: { ...content, name: content.name.trim() },
    createdAt: now,
    updatedAt: now,
  };
  templates = [...templates, template].sort((a, b) => a.content.name.localeCompare(b.content.name));
  return ok(template);
}

const find = (id: string) => templates.find((template) => template.id === id);

/** The meeting's choice and why. */
function choice(meeting: MeetingId): Generated.TemplateChoice | null {
  const row = mockDatabase.meeting(meeting);
  if (row.status === 'error') return null;
  const picked = find(chosen.get(meeting) ?? '');
  if (picked !== undefined) return { template: picked, reason: { kind: 'chosen' } };
  for (const template of templates) {
    const rule = template.content.autoRules.find(
      (candidate) =>
        candidate.titlePattern !== null &&
        new RegExp(candidate.titlePattern, 'i').test(row.data.title),
    );
    if (rule !== undefined) return { template, reason: { kind: 'rule', rule: rule.label } };
  }
  const general = find('general') ?? templates[0];
  return general === undefined ? null : { template: general, reason: { kind: 'fallback' } };
}

export const commands = {
  async list() {
    await latency();
    return ok([...templates]);
  },
  async get(id) {
    await latency();
    const template = find(id);
    return template === undefined ? notFound(id) : ok(template);
  },
  async create(content) {
    await latency();
    return create(content);
  },
  async update(id, content) {
    await latency();
    const template = find(id);
    if (template === undefined) return notFound(id);
    if (template.isBuiltin) return readOnly;
    const updated = { ...template, content, updatedAt: new Date().toISOString() };
    templates = templates.map((candidate) => (candidate.id === id ? updated : candidate));
    return ok(updated);
  },
  async duplicate(id) {
    await latency();
    const template = find(id);
    if (template === undefined) return notFound(id);
    return create({ ...template.content, name: `${template.content.name} (copy)` });
  },
  async delete(id) {
    await latency();
    const template = find(id);
    if (template === undefined) return notFound(id);
    if (template.isBuiltin) return readOnly;
    templates = templates.filter((candidate) => candidate.id !== id);
    return ok(null);
  },
  async importJson(json) {
    await latency();
    try {
      const document = JSON.parse(json) as { template?: Generated.TemplateContent };
      if (document.template === undefined) throw new TypeError('not a template');
      return create(document.template);
    } catch {
      return failure<Template>({ code: 'invalid', message: 'this file is not a Deino template' });
    }
  },
  async exportJson(id) {
    await latency();
    const template = find(id);
    if (template === undefined) return notFound(id);
    return ok(
      JSON.stringify({ format: 'deino.template', version: 1, template: template.content }, null, 2),
    );
  },
  async preview(content, meeting) {
    await latency();
    const row = mockDatabase.meeting(meeting);
    if (row.status === 'error') return row;
    const sections = content.sections.map(
      (s) => `- **For the '${s.title}' section:** ${s.instructions}`,
    );
    return ok({
      blocks: [
        { heading: 'This meeting', body: `Title: ${row.data.title}` },
        { heading: 'Template', body: `${content.name}. ${content.meetingContext}`.trim() },
        { heading: 'Sections', body: sections.join('\n') },
      ],
    });
  },
  async meetingTemplate(meeting) {
    await latency();
    const result = choice(meeting);
    return result === null
      ? failure({ code: 'not_found', message: `meeting ${meeting} was not found` })
      : ok(result);
  },
  async setMeetingTemplate(meeting, template) {
    await latency();
    if (template !== null && find(template) === undefined) return notFound(template);
    if (template === null) chosen.delete(meeting);
    else chosen.set(meeting, template);
    const result = choice(meeting);
    return result === null
      ? failure({ code: 'not_found', message: `meeting ${meeting} was not found` })
      : ok(result);
  },
} satisfies typeof Generated.commands;

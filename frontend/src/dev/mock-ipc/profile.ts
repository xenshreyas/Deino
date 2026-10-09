/**
 * Mock of `@/lib/ipc/generated/profile`: saving normalises like Rust (trim,
 * lower-case, drop blanks and repeats) and writes the settings mock, which
 * announces `SettingsChanged`; the context is a short rendering of it.
 */
import type * as Generated from '@/lib/ipc/generated/profile';

import { latency } from './latency';
import { failure, ok } from './mock-database';
import { commands as settings } from './settings';

/** The trimmed, non-blank entries of `items`, each once. */
function unique(items: readonly string[], key: (item: string) => string): string[] {
  const kept: string[] = [];
  for (const item of items.map((entry) => entry.trim())) {
    if (item !== '' && !kept.some((other) => key(other) === key(item))) kept.push(item);
  }
  return kept;
}

/** The profile `input` normalises to, or the 1-based position of a bad address. */
function normalize(input: Generated.ProfileInput): Generated.ProfileSettings | number {
  const emails = unique(input.emails, (email) => email).map((email) => email.toLowerCase());
  const bad = emails.findIndex((email) => !/^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(email));
  if (bad >= 0) return bad + 1;
  return {
    name: input.name.trim(),
    emails,
    role: input.role.trim(),
    company: input.company.trim(),
    focusAreas: unique(input.focusAreas, (area) => area.toLowerCase()),
    writingStyle: input.writingStyle.trim(),
    context: input.context.trim(),
  };
}

export const commands = {
  async update(input) {
    await latency();
    const profile = normalize(input);
    if (typeof profile === 'number') {
      return failure({ code: 'invalid', message: `email address ${String(profile)} is not valid` });
    }
    await settings.patch({ profile });
    return ok(profile);
  },
  async context() {
    await latency();
    const saved = await settings.get();
    if (saved.status === 'error') return saved;
    const { name, role, company } = saved.data.profile;
    const lines = [
      name && `- Name: ${name}`,
      role && `- Role: ${[role, company].filter(Boolean).join(' at ')}`,
    ];
    const facts = lines.filter((line) => line !== '');
    return ok(
      facts.length === 0 ? null : `About the user, who these notes are for:\n${facts.join('\n')}`,
    );
  },
} satisfies typeof Generated.commands;

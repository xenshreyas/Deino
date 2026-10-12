/**
 * Pure helpers over a draft's text: its `[placeholders]` (the same rule as
 * Rust's `deino_llm::followup::placeholders`) for highlighting, and the
 * comma-separated address fields the editor shows.
 */
import type { DraftFields, FollowUpDraft, FollowUpEdit } from './types';

/** A run of the body: plain text, or a `[placeholder]` with its brackets. */
export interface TextRun {
  text: string;
  placeholder: boolean;
}

/** The longest placeholder, in characters, between its brackets. */
const MAX_PLACEHOLDER = 60;

/**
 * `[gap]` runs on one line, 1–60 characters, no nested brackets, not a
 * Markdown link (`[text](url)`).
 */
const PLACEHOLDER = new RegExp(`\\[[^\\[\\]\\n]{1,${MAX_PLACEHOLDER.toString()}}\\](?!\\()`, 'g');

/** `text` split into plain runs and placeholders, in order. */
export function textRuns(text: string): TextRun[] {
  const runs: TextRun[] = [];
  let last = 0;
  for (const match of text.matchAll(PLACEHOLDER)) {
    const start = match.index;
    if (start > last) runs.push({ text: text.slice(last, start), placeholder: false });
    runs.push({ text: match[0], placeholder: true });
    last = start + match[0].length;
  }
  if (last < text.length) runs.push({ text: text.slice(last), placeholder: false });
  return runs;
}

/** How many placeholders `text` still has. */
export function placeholderCount(text: string): number {
  return textRuns(text).filter((run) => run.placeholder).length;
}

/** The addresses typed in a field: split on commas, semicolons and spaces. */
export function splitAddresses(field: string): string[] {
  return field
    .split(/[,;\s]+/)
    .map((address) => address.trim())
    .filter((address) => address !== '');
}

/** A draft as the editor's fields. */
export function fieldsOf(draft: FollowUpDraft): DraftFields {
  return {
    subject: draft.subject,
    body: draft.body,
    to: draft.recipients.to.join(', '),
    cc: draft.recipients.cc.join(', '),
    bcc: draft.recipients.bcc.join(', '),
  };
}

/** The editor's fields as the edit Rust checks and stores. */
export function editOf(fields: DraftFields): FollowUpEdit {
  return {
    subject: fields.subject,
    body: fields.body,
    to: splitAddresses(fields.to),
    cc: splitAddresses(fields.cc),
    bcc: splitAddresses(fields.bcc),
  };
}

/** True when the fields differ from the stored draft. */
export function isDirty(fields: DraftFields, draft: FollowUpDraft): boolean {
  const stored = fieldsOf(draft);
  return (Object.keys(stored) as (keyof DraftFields)[]).some((key) => stored[key] !== fields[key]);
}

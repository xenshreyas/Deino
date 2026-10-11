/**
 * A small stand-in for `deino-export` in `pnpm dev:mock`: enough of each
 * format, rendered from a fixture meeting, for the export dialog's preview
 * to show realistic content. The real renderers and their golden files are
 * in Rust; this never needs to match them byte for byte.
 */
import type * as Generated from '@/lib/ipc/generated/export';

import { mockInsights } from './database';
import type { FixtureMeeting } from './fixture-schema';

type Sections = Generated.ExportSections;

/** "2026-10-08". */
function day(iso: string): string {
  return iso.slice(0, 10);
}

/** `mm:ss` from milliseconds. */
function clock(ms: number): string {
  const seconds = Math.floor(ms / 1000);
  const pad = (value: number) => String(value).padStart(2, '0');
  return `${pad(Math.floor(seconds / 60))}:${pad(seconds % 60)}`;
}

function attendees(meeting: FixtureMeeting): string[] {
  return meeting.attendees.filter(({ rsvp }) => rsvp !== 'declined').map(({ name }) => name);
}

function speakerName(meeting: FixtureMeeting, label: string): string {
  const email = meeting.speakers.find((speaker) => speaker.label === label)?.attendee;
  return meeting.attendees.find((attendee) => attendee.email === email)?.name ?? label;
}

function transcriptLines(meeting: FixtureMeeting): string[] {
  return meeting.segments.map(
    (segment) =>
      `[${clock(segment.startMs)}] ${speakerName(meeting, segment.speaker)}: ${segment.text}`,
  );
}

/** The suggested file name, as `ExportService` builds it. */
export function fileName(meeting: FixtureMeeting, format: Generated.ExportFormat): string {
  const extension = format === 'markdown' ? 'md' : format === 'text' ? 'txt' : format;
  return `${day(meeting.startedAt)} ${meeting.title}.${extension}`;
}

/** The meeting as Markdown with frontmatter. */
export function markdown(meeting: FixtureMeeting, sections: Sections): string {
  const names = attendees(meeting);
  const parts = [
    [
      '---',
      `title: "${meeting.title}"`,
      `date: ${meeting.startedAt.slice(0, 19)}`,
      'attendees:',
      ...names.map((name) => `  - "${name}"`),
      `deino-id: "${meeting.id}"`,
      '---',
    ].join('\n'),
    `# ${meeting.title}`,
  ];
  if (sections.attendees)
    parts.push(`## Attendees\n\n${names.map((name) => `- ${name}`).join('\n')}`);
  if (sections.notes)
    parts.push(
      `## Notes\n\n${meeting.groundTruth.decisions.map((line) => `- ${line}`).join('\n')}`,
    );
  if (sections.myNotes && meeting.userNotes !== null)
    parts.push(`## My notes\n\n${meeting.userNotes}`);
  if (sections.actionItems)
    parts.push(
      `## Action items\n\n${meeting.groundTruth.actionItems.map(({ title }) => `- [ ] ${title}`).join('\n')}`,
    );
  if (sections.insights) parts.push(...insightsMarkdown(meeting));
  if (sections.transcript) parts.push(`## Transcript\n\n${transcriptLines(meeting).join('\n\n')}`);
  return `${parts.join('\n\n')}\n`;
}

/** The meeting's answered insights as a Markdown section (none when there are none). */
function insightsMarkdown(meeting: FixtureMeeting): string[] {
  const answered = mockInsights.answeredOn(meeting.id);
  if (answered.length === 0) return [];
  const blocks = answered.map(({ name, answers }) => {
    const lines = answers.map((answer) =>
      answer.value.kind === 'text'
        ? `- ${answer.value.text}`
        : `- ${answer.value.fields
            .map(
              (field) => `${field.name}: ${field.value === null ? '' : String(field.value.value)}`,
            )
            .join(' · ')}`,
    );
    return `### ${name}\n\n${lines.join('\n')}`;
  });
  return [`## Insights\n\n${blocks.join('\n\n')}`];
}

/** The meeting as plain text. */
export function text(meeting: FixtureMeeting, sections: Sections): string {
  return markdown(meeting, sections)
    .replace(/^---[\s\S]*?---\n\n/, '')
    .replace(/^#+ /gm, '')
    .replace(/^- \[ \] /gm, '[ ] ');
}

/** The transcript as subtitles. */
export function subtitles(meeting: FixtureMeeting, vtt: boolean): string {
  const time = (ms: number) =>
    `00:${clock(ms)}${vtt ? '.' : ','}${String(ms % 1000).padStart(3, '0')}`;
  const cues = meeting.segments.map((segment, index) => {
    const range = `${time(segment.startMs)} --> ${time(segment.endMs)}`;
    const speaker = speakerName(meeting, segment.speaker);
    return vtt
      ? `${range}\n<v ${speaker}>${segment.text}\n`
      : `${String(index + 1)}\n${range}\n${speaker}: ${segment.text}\n`;
  });
  return vtt ? `WEBVTT\n\n${cues.join('\n')}` : cues.join('\n');
}

function escapeHtml(value: string): string {
  return value.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

/** The document preview fragment PDF and DOCX show. */
export function html(meeting: FixtureMeeting, sections: Sections): string {
  const list = (items: string[]) =>
    `<ul>${items.map((item) => `<li>${escapeHtml(item)}</li>`).join('')}</ul>`;
  const parts = [
    `<h1>${escapeHtml(meeting.title)}</h1><p data-role="meta">${day(meeting.startedAt)}</p>`,
  ];
  if (sections.attendees)
    parts.push(`<h2>Attendees</h2><p>${escapeHtml(attendees(meeting).join(', '))}</p>`);
  if (sections.notes) parts.push(`<h2>Notes</h2>${list(meeting.groundTruth.decisions)}`);
  if (sections.actionItems)
    parts.push(
      `<h2>Action items</h2>${list(meeting.groundTruth.actionItems.map(({ title }) => `☐ ${title}`))}`,
    );
  if (sections.transcript)
    parts.push(
      `<h2>Transcript</h2>${transcriptLines(meeting)
        .map((line) => `<p>${escapeHtml(line)}</p>`)
        .join('')}`,
    );
  return `<div>${parts.join('')}</div>`;
}

/**
 * The notepad's `/template` command (NOT-04, Granola's "/template", 17 Aug
 * 2026): typed before or during a meeting, it sets the meeting's template
 * and inserts the template's section headings as a scaffold to write under.
 *
 * A self-contained TipTap suggestion: NOT-01's editor registers it with
 * `@tiptap/suggestion` (`Suggestion({ editor, ...templateSlashSuggestion(deps) })`
 * inside an `Extension.create`) and renders {@link TemplateSlashList} as the
 * menu. The types below are the structural subset of TipTap's API that the
 * command touches, so this module needs no TipTap import and TipTap's own
 * `Editor` and `Range` satisfy them.
 */
import type { Template, TemplateId } from '../types';

/** The word after `/` that opens the template list. */
export const TEMPLATE_COMMAND = 'template';
/** The most templates the menu lists at once. */
export const MAX_SLASH_ITEMS = 8;

/** A document range (TipTap's `Range`). */
export interface SlashRange {
  from: number;
  to: number;
}

/** A TipTap JSON node, as far as the scaffold needs. */
export interface ScaffoldNode {
  type: 'heading' | 'paragraph' | 'text';
  attrs?: { level: 2 };
  content?: ScaffoldNode[];
  text?: string;
}

/** The part of TipTap's command chain the command uses. */
export interface ScaffoldChain {
  focus: () => ScaffoldChain;
  deleteRange: (range: SlashRange) => ScaffoldChain;
  insertContent: (content: ScaffoldNode[]) => ScaffoldChain;
  run: () => boolean;
}

/** The part of TipTap's `Editor` the command uses. */
export interface ScaffoldEditor {
  chain: () => ScaffoldChain;
}

/** What the command needs from the app. */
export interface TemplateSlashDeps {
  /** The templates to offer (the cached list). */
  templates: () => readonly Template[];
  /** Saves the meeting's template (`set_meeting_template`). */
  setMeetingTemplate: (template: TemplateId) => void;
}

/** The suggestion options to spread into `Suggestion({ editor, ... })`. */
export interface TemplateSlashSuggestion {
  char: '/';
  startOfLine: false;
  items: (props: { query: string }) => Template[];
  command: (props: { editor: ScaffoldEditor; range: SlashRange; props: Template }) => void;
}

/**
 * What to filter templates by, or `null` when the query is not (a prefix
 * of) `template`: "templ" → "", "template sales" → "sales".
 */
export function templateQuery(query: string): string | null {
  const [command = '', ...rest] = query.trimStart().split(/\s+/);
  if (command === '' || !TEMPLATE_COMMAND.startsWith(command.toLowerCase())) return null;
  if (rest.length > 0 && command.toLowerCase() !== TEMPLATE_COMMAND) return null;
  return rest.join(' ').trim().toLowerCase();
}

/** Templates matching `filter`, names starting with it first, at most {@link MAX_SLASH_ITEMS}. */
export function matchTemplates(templates: readonly Template[], filter: string): Template[] {
  const matches = templates.filter(({ content }) =>
    `${content.name} ${content.description}`.toLowerCase().includes(filter),
  );
  const starts = (template: Template) => template.content.name.toLowerCase().startsWith(filter);
  return [...matches.filter(starts), ...matches.filter((t) => !starts(t))].slice(
    0,
    MAX_SLASH_ITEMS,
  );
}

/** One second-level heading per section, each followed by an empty paragraph to type in. */
export function scaffoldContent(template: Template): ScaffoldNode[] {
  return template.content.sections.flatMap((section) => [
    { type: 'heading', attrs: { level: 2 }, content: [{ type: 'text', text: section.title }] },
    { type: 'paragraph' },
  ]);
}

/** The `/template` suggestion (see the module docs). */
export function templateSlashSuggestion(deps: TemplateSlashDeps): TemplateSlashSuggestion {
  return {
    char: '/',
    startOfLine: false,
    items: ({ query }) => {
      const filter = templateQuery(query);
      return filter === null ? [] : matchTemplates(deps.templates(), filter);
    },
    command: ({ editor, range, props: template }) => {
      deps.setMeetingTemplate(template.id);
      editor.chain().focus().deleteRange(range).insertContent(scaffoldContent(template)).run();
    },
  };
}

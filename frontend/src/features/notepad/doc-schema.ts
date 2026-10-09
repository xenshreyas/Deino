/**
 * The boundary between the editor and IPC: TipTap's `getJSON()` is loosely
 * typed (`JSONContent`), the `notes` commands take the generated `NoteDoc`.
 * This zod schema mirrors `deino-doc`'s model (`crates/deino-doc/src/model.rs`)
 * and is checked against the generated types, so a drift fails typecheck.
 * Parsing strips attributes Deino does not model (a link's `target`, a
 * mention's `mentionSuggestionChar`) and fills TipTap's omitted `content`
 * and `marks`. Rust validates again; this keeps the types honest.
 */
import { z } from 'zod';

import type { PersonId, SegmentId } from '@/lib/ipc/generated/notes';

import type { Block, Inline, Mark, NoteDoc } from './types';

/** IDs minted by Rust; the UUID check is the validation that brands them. */
const personId = z
  .string()
  .uuid()
  .transform((id) => id as PersonId);
const segmentId = z
  .string()
  .uuid()
  .transform((id) => id as SegmentId);

const markSchema: z.ZodType<Mark, z.ZodTypeDef, unknown> = z.discriminatedUnion('type', [
  z.object({
    type: z.literal('origin'),
    attrs: z.object({ author: z.enum(['user', 'ai']), edited: z.boolean().default(false) }),
  }),
  z.object({
    type: z.literal('link'),
    attrs: z.object({ href: z.string(), title: z.string().nullable().default(null) }),
  }),
  z.object({ type: z.literal('bold') }),
  z.object({ type: z.literal('italic') }),
  z.object({ type: z.literal('underline') }),
  z.object({ type: z.literal('strike') }),
  z.object({ type: z.literal('code') }),
]);

const marks = z.array(markSchema).default([]);

const inlineSchema: z.ZodType<Inline, z.ZodTypeDef, unknown> = z.discriminatedUnion('type', [
  z.object({ type: z.literal('text'), text: z.string(), marks }),
  z.object({ type: z.literal('hardBreak'), marks }),
  z.object({
    type: z.literal('mention'),
    attrs: z.object({ id: personId, label: z.string() }),
    marks,
  }),
  z.object({
    type: z.literal('citation'),
    attrs: z.object({ segmentId, startMs: z.number().int().nonnegative() }),
    marks,
  }),
]);

const inlines = z.array(inlineSchema).default([]);

const blockSchema: z.ZodType<Block, z.ZodTypeDef, unknown> = z.lazy(() =>
  z.discriminatedUnion('type', [
    z.object({ type: z.literal('paragraph'), content: inlines }),
    z.object({
      type: z.literal('heading'),
      attrs: z.object({ level: z.number().int() }),
      content: inlines,
    }),
    z.object({ type: z.literal('bulletList'), content: z.array(listItemSchema) }),
    z.object({
      type: z.literal('orderedList'),
      attrs: z.object({ start: z.number().int().default(1) }).default({ start: 1 }),
      content: z.array(listItemSchema),
    }),
    z.object({ type: z.literal('taskList'), content: z.array(taskItemSchema) }),
    z.object({ type: z.literal('blockquote'), content: z.array(blockSchema) }),
    z.object({
      type: z.literal('codeBlock'),
      attrs: z
        .object({ language: z.string().nullable().default(null) })
        .default({ language: null }),
      content: inlines,
    }),
    z.object({ type: z.literal('horizontalRule') }),
    z.object({
      type: z.literal('image'),
      attrs: z.object({
        src: z.string(),
        alt: z.string().nullable().default(null),
        title: z.string().nullable().default(null),
      }),
    }),
  ]),
);

const blocks = z.array(blockSchema).default([]);

const listItemSchema = z.object({ type: z.literal('listItem'), content: blocks });

const taskItemSchema = z.object({
  type: z.literal('taskItem'),
  attrs: z.object({ checked: z.boolean().default(false) }).default({ checked: false }),
  content: blocks,
});

/** The schema of a whole document. */
export const noteDocSchema: z.ZodType<NoteDoc, z.ZodTypeDef, unknown> = z.object({
  type: z.literal('doc'),
  content: blocks,
});

/** Reads the editor's JSON as a `NoteDoc`; throws a `ZodError` on a shape Rust would refuse. */
export function toNoteDoc(json: unknown): NoteDoc {
  return noteDocSchema.parse(json);
}

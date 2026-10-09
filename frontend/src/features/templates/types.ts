/**
 * The templates feature's types. Shapes shared with Rust are generated
 * (`src-tauri/src/bindings.rs`), never written by hand.
 */
import type { TemplateId } from '@/lib/ipc/generated/templates';

export type {
  Audience,
  AutoRule,
  ChoiceReason,
  CountRange,
  PromptBlock,
  SectionFormat,
  Template,
  TemplateChoice,
  TemplateContent,
  TemplateId,
  TemplatePreview,
  TemplateSection,
} from '@/lib/ipc/generated/templates';

/** The ID of the General template, the fallback (`GENERAL_TEMPLATE_ID` in Rust). */
export const GENERAL_TEMPLATE_ID = asTemplateId('general');

/**
 * Brands a template ID that arrived as a plain string (a URL parameter).
 * Template IDs are opaque text minted by Rust, so the only check is that
 * there is one.
 */
export function asTemplateId(raw: string): TemplateId {
  if (raw.trim() === '') throw new TypeError('A template ID cannot be empty.');
  return raw as TemplateId;
}

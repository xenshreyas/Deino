/** Settings → Import's words (CAP-12). */
import type { ImportPhase, ImportSource } from './types';

/** How each source is named and what it needs. */
export const sourceCopy = {
  granola: {
    name: 'Granola',
    hint: 'A personal API key from Granola → Settings → Connectors → API keys.',
    placeholder: 'grn_…',
  },
  circleback: {
    name: 'Circleback',
    hint: 'A read-only key from Circleback → Settings → API keys.',
    placeholder: 'cb_…',
  },
  fireflies: {
    name: 'Fireflies',
    hint: 'Your API key from Fireflies → Integrations → Fireflies API.',
    placeholder: 'API key',
  },
  fathom: {
    name: 'Fathom',
    hint: 'An API key from Fathom → Settings → API access.',
    placeholder: 'API key',
  },
  folder: {
    name: 'A folder',
    hint: 'JSON files in the local API’s import shape, and Markdown notes.',
    placeholder: '',
  },
} as const satisfies Record<ImportSource, { name: string; hint: string; placeholder: string }>;

/** Where the job is, in words. */
export const phaseCopy = {
  idle: 'Not started',
  listing: 'Counting meetings…',
  importing: 'Importing…',
  finished: 'Import finished',
  cancelled: 'Import stopped',
  failed: 'Import failed',
} as const satisfies Record<ImportPhase, string>;

export const importCopy = {
  title: 'Import from other note-takers',
  description:
    'Bring your meetings into Deino with notes, transcripts, attendees and action items. Keys stay in the Keychain; a meeting is never imported twice.',
  folderTitle: 'Import a folder',
  folderDescription: 'Exported meetings already on this Mac.',
  keyLabel: (name: string) => `${name} API key`,
  saveKey: 'Save key',
  forgetKey: 'Forget key',
  keySaved: 'Key saved in the Keychain',
  keyLocked: 'macOS would not share the saved key; enter it again',
  chooseFolder: 'Choose folder…',
  noFolder: 'No folder chosen',
  preview: 'Check',
  import: 'Import',
  previewing: 'Checking…',
  previewResult: (fresh: number, already: number) =>
    already === 0
      ? `${countLabel(fresh)} to import`
      : `${countLabel(fresh)} to import, ${String(already)} already here`,
  nothingNew: 'Everything here is already imported',
  progress: (done: number, total: number) => `${String(done)} of ${String(total)}`,
  summary: (imported: number, skipped: number, failed: number) =>
    [
      `${countLabel(imported)} imported`,
      skipped > 0 ? `${String(skipped)} already here` : null,
      failed > 0 ? `${String(failed)} could not be read` : null,
    ]
      .filter((part) => part !== null)
      .join(', '),
  cancel: 'Stop',
  offline: 'Imports reach each provider directly; offline mode blocks them.',
  loadFailed: 'Import sources could not be loaded.',
} as const;

/** "1 meeting", "3 meetings". */
function countLabel(count: number): string {
  return count === 1 ? '1 meeting' : `${String(count)} meetings`;
}

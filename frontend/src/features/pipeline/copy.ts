/** Every string the pipeline feature shows, in one place (§3A). */
import type { JobStatus } from './types';

export const copy = {
  label: 'Processing',
  retry: 'Retry',
  retryStage: (stage: string) => `Retry ${stage.toLowerCase()}`,
  retryFailed: 'That step couldn’t be restarted.',
  retranscribeFailed: 'Couldn’t re-transcribe this meeting',
  loadFailed: 'Processing status is unavailable.',
  retrying: 'Retrying soon',
  detailsTitle: 'Processing details',
  detailsDescription: 'Every step Deino ran after this meeting, and how each one went.',
  detailsEmpty: 'Nothing has run for this meeting yet.',
  close: 'Close',
  /** After a running stage's line when it says how far it is: " · 40%". */
  percent: (percent: number) => ` · ${String(Math.round(percent))}%`,
} as const;

/** What each status reads as, in Processing details. */
export const statusLabels: Record<JobStatus, string> = {
  pending: 'Waiting',
  running: 'Running',
  succeeded: 'Done',
  skipped: 'Skipped',
  failed: 'Failed',
  cancelled: 'Cancelled',
};

/**
 * Every stage `deino_pipeline::registry` registers, in run order. A stage
 * added in Rust gets a name here too, or {@link stageCopy} fails to compile.
 */
export const KNOWN_STAGES = [
  'finalize_audio',
  'screen_video',
  'final_transcription',
  'diarize',
  'identify_speakers',
  'vocab_fixups',
  'screen_context',
  'enhance_notes',
  'extract_action_items',
  'auto_title',
  'auto_tag',
  'auto_folder',
  'index',
  'insights',
  'update_entities',
  'automations',
  'follow_up_draft',
  'auto_export',
] as const;

/** A stage the UI knows by name. */
export type KnownStage = (typeof KNOWN_STAGES)[number];

/** A stage's name, what it says while it runs, and what it says when it fails. */
interface StageWords {
  label: string;
  running: string;
  failed: string;
}

/** The words for every stage. */
const stageCopy: Record<KnownStage, StageWords> = {
  finalize_audio: words('Audio', 'Saving the audio…', 'The audio couldn’t be saved.'),
  screen_video: words(
    'Screen video',
    'Saving the screen video…',
    'The screen video couldn’t be saved.',
  ),
  final_transcription: words('Transcript', 'Transcribing…', 'The transcript couldn’t be finished.'),
  diarize: words('Speakers', 'Telling voices apart…', 'Voices couldn’t be told apart.'),
  identify_speakers: words('Names', 'Naming speakers…', 'Speakers couldn’t be named.'),
  vocab_fixups: words('Words and names', 'Fixing names…', 'Names couldn’t be fixed.'),
  screen_context: words(
    'Screen text',
    'Reading the shared screen…',
    'The shared screen couldn’t be read.',
  ),
  enhance_notes: words('Notes', 'Writing notes…', 'Notes couldn’t be written.'),
  extract_action_items: words(
    'Action items',
    'Finding action items…',
    'Action items couldn’t be found.',
  ),
  auto_title: words('Title', 'Choosing a title…', 'A title couldn’t be chosen.'),
  auto_tag: words('Tags', 'Adding tags…', 'Tags couldn’t be added.'),
  auto_folder: words('Folders', 'Filing the meeting…', 'The meeting couldn’t be filed.'),
  index: words('Search', 'Making it searchable…', 'The meeting couldn’t be made searchable.'),
  insights: words('Insights', 'Running insights…', 'Insights couldn’t run.'),
  update_entities: words(
    'People and companies',
    'Updating people…',
    'People and companies couldn’t be updated.',
  ),
  automations: words('Automations', 'Running automations…', 'Automations couldn’t run.'),
  follow_up_draft: words(
    'Follow-up email',
    'Drafting a follow-up…',
    'The follow-up email couldn’t be drafted.',
  ),
  auto_export: words('Export', 'Exporting…', 'The meeting couldn’t be exported.'),
};

function words(label: string, running: string, failed: string): StageWords {
  return { label, running, failed };
}

/** Whether `stage` is one the UI has words for. */
function isKnown(stage: string): stage is KnownStage {
  return (KNOWN_STAGES as readonly string[]).includes(stage);
}

/** The words for `stage`; a stage newer than this build reads as plain "Processing". */
function wordsFor(stage: string): StageWords {
  if (isKnown(stage)) return stageCopy[stage];
  return words(copy.label, 'Processing…', 'A processing step couldn’t finish.');
}

/** The readable name of `stage` ("Notes"). */
export function stageLabel(stage: string): string {
  return wordsFor(stage).label;
}

/** What the meeting header says while `stage` runs ("Writing notes…"). */
export function stageRunning(stage: string): string {
  return wordsFor(stage).running;
}

/** What the meeting header says when `stage` failed ("Notes couldn’t be written."). */
export function stageFailed(stage: string): string {
  return wordsFor(stage).failed;
}

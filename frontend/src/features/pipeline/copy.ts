/** Every string the pipeline feature shows, in one place (§3A). */
import type { JobStatus } from './types';

export const copy = {
  label: 'Processing',
  retry: 'Retry',
  retryStage: (stage: string) => `Retry ${stage}`,
  retryFailed: 'The step could not be restarted.',
  loadFailed: 'Processing status is unavailable.',
  retrying: 'Retrying soon',
} as const;

/** What each status reads as. */
export const statusLabels: Record<JobStatus, string> = {
  pending: 'Waiting',
  running: 'Running',
  succeeded: 'Done',
  skipped: 'Skipped',
  failed: 'Failed',
  cancelled: 'Cancelled',
};

/** Readable names of the stages; unknown stages show their ID. */
const stageLabels: Partial<Record<string, string>> = {
  final_transcription: 'Transcript',
  diarize: 'Speakers',
  identify_speakers: 'Names',
  vocab_fixups: 'Vocabulary',
  screen_context: 'Screen text',
  enhance_notes: 'Notes',
  extract_action_items: 'Action items',
  auto_title: 'Title',
  auto_tag: 'Tags',
  auto_folder: 'Folders',
  insights: 'Insights',
  follow_up_draft: 'Follow-up email',
  auto_export: 'Export',
};

/** The readable name of `stage`. */
export function stageLabel(stage: string): string {
  return stageLabels[stage] ?? stage.replaceAll('_', ' ');
}

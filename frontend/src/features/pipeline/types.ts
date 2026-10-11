/** The pipeline feature's types, generated from Rust by `bindings.rs`. */
import type { StageId } from '@/lib/ipc/generated/pipeline';

export type {
  JobStatus,
  MeetingId,
  PipelineProgress,
  StageId,
  StageStatus,
} from '@/lib/ipc/generated/pipeline';

/**
 * The stage that writes the meeting's notes, re-run when names change.
 * NOT-02's `enhance_notes` replaces `legacy_summary`.
 */
export const NOTES_STAGE = 'legacy_summary' as StageId;

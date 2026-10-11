/** Types of the recording feature: the generated ones, and the lines it shows. */
import type { Channel, MeetingId, SegmentId } from '@/lib/ipc/generated/recording';

export type {
  Channel,
  HealthKind,
  HealthNotice,
  RecordingPhase,
  RecordingSnapshot,
  Segment,
  SegmentId,
  SegmentUpdate,
  SegmentUpdateKind,
} from '@/lib/ipc/generated/recording';

/** One line of the live transcript, saved or still being spoken. */
export interface TranscriptLine {
  id: SegmentId;
  meetingId: MeetingId;
  /** Me (`mic`) or Them (`system`). */
  channel: Channel;
  /** Milliseconds from the start of the recording. */
  start: number;
  text: string;
  /** Still being spoken: the text may grow. */
  open: boolean;
}

/**
 * The enhanced notes' types, generated from `deino_core::enhance` and
 * re-exported here.
 */
export type {
  Block,
  CitedSegment,
  doc as DraftDoc,
  EnhancedNotesView,
  Inline,
  Mark,
  NotesDraft,
  NotesPhase,
  NotesRun,
  NotesRunEnd,
  NotesRunUpdate,
  RegenerateReason,
  SavedEnhancedNotes,
  SegmentId,
  VersionInfo,
} from '@/lib/ipc/generated/enhance';

/** A citation as the document carries it (`deino_doc::CitationAttrs`). */
export interface CitationRef {
  segmentId: string;
  startMs: number;
}

/** A section heading of the notes, for the table of contents. */
export interface TocEntry {
  /** Its position among the document's headings. */
  index: number;
  text: string;
  level: number;
}

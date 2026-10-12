/**
 * The edit-by-asking types, generated from `deino_core::notes::edit` and
 * re-exported here.
 */
export type {
  AcceptedEdit,
  BlockChange,
  DiffBlock,
  DiffSegment,
  EditMethod,
  EditProposal,
  EditRequest,
  SegmentChange,
  TextKind,
} from '@/lib/ipc/generated/notes-edit';

/** How the preview lays out the changes. */
export type DiffLayout = 'inline' | 'split';

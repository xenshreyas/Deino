/** Types of the export feature, generated from Rust. */
import type { ExportSections } from '@/lib/ipc/generated/export';

export type {
  CopyKind,
  ExportFormat,
  ExportPreview,
  ExportSections,
  FullExportState,
  FullExportStatus,
  MeetingId,
  PreviewKind,
  SavedExport,
  SavedExports,
} from '@/lib/ipc/generated/export';
export type { ExportSettings } from '@/lib/ipc/generated/settings';

/** A section switch of the export dialog and Settings → Export. */
export type SectionKey = keyof ExportSections;

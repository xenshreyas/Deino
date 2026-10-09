/**
 * Typed calls to the `export` plugin. Each resolves to the data or throws
 * an `IpcError`. No call takes a path: Rust opens every dialog.
 */
import { commands } from '@/lib/ipc/generated/export';
import { unwrap } from '@/lib/ipc/result';

import type {
  CopyKind,
  ExportFormat,
  ExportPreview,
  ExportSections,
  FullExportStatus,
  MeetingId,
  SavedExport,
} from './types';

export const exportApi = {
  /** What exporting the meeting as `format` would write. */
  preview: (
    meetingId: MeetingId,
    format: ExportFormat,
    sections: ExportSections,
  ): Promise<ExportPreview> => unwrap(commands.preview(meetingId, format, sections)),
  /** Opens the save dialog and writes the file; `null` when cancelled. */
  save: (
    meetingId: MeetingId,
    format: ExportFormat,
    sections: ExportSections,
  ): Promise<SavedExport | null> => unwrap(commands.save(meetingId, format, sections)),
  /** Puts the meeting on the clipboard. */
  copy: (meetingId: MeetingId, kind: CopyKind): Promise<null> =>
    unwrap(commands.copy(meetingId, kind)),
  /** Opens an email draft of the notes. */
  email: (meetingId: MeetingId): Promise<null> => unwrap(commands.email(meetingId)),
  /** Opens the folder dialog for auto-export; the chosen folder, or `null`. */
  chooseFolder: (): Promise<string | null> => unwrap(commands.chooseFolder()),
  /** Opens the save dialog and starts the account export; `false` when cancelled. */
  startFullExport: (includeAudio: boolean): Promise<boolean> =>
    unwrap(commands.startFullExport(includeAudio)),
  /** Cancels the running account export. */
  cancelFullExport: (): Promise<null> => unwrap(commands.cancelFullExport()),
  /** Where the account export is. */
  fullExportStatus: (): Promise<FullExportStatus> => unwrap(commands.fullExportStatus()),
};

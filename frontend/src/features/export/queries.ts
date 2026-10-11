/**
 * TanStack Query hooks of the export feature.
 *
 * The preview reads the meeting through every table an export draws on, so
 * `DbChanged` re-renders it when the notes, transcript or people change. The
 * account export's status is cached once and kept current by its typed
 * `FullExportProgress` event.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { events } from '@/lib/ipc/generated/export';
import type { Table } from '@/lib/ipc/generated/shared';
import { useIpcEvent } from '@/lib/ipc/events';

import { exportApi } from './api';
import type { CopyKind, ExportFormat, ExportSections, FullExportStatus, MeetingId } from './types';

/** Every table an export reads. */
const EXPORT_TABLES = [
  'meetings',
  'meeting_notes',
  'enhanced_notes',
  'transcript_segments',
  'speakers',
  'meeting_attendees',
  'people',
  'meeting_tags',
  'tags',
  'meeting_folders',
  'folders',
  'action_items',
] as const satisfies readonly Table[];

/** Query keys; every key starts with `exportKeys.all`. */
export const exportKeys = {
  all: ['export'] as const,
  preview: (meetingId: MeetingId, format: ExportFormat, sections: ExportSections) =>
    [...exportKeys.all, 'preview', meetingId, format, sections] as const,
  fullStatus: () => [...exportKeys.all, 'full-status'] as const,
};

/** What exporting the meeting as `format` would write. */
export const previewQuery = (
  meetingId: MeetingId,
  format: ExportFormat,
  sections: ExportSections,
) =>
  queryOptions({
    queryKey: exportKeys.preview(meetingId, format, sections),
    queryFn: () => exportApi.preview(meetingId, format, sections),
    meta: { tables: EXPORT_TABLES },
  });

/** The account export's status. */
export const fullExportStatusQuery = () =>
  queryOptions({ queryKey: exportKeys.fullStatus(), queryFn: exportApi.fullExportStatus });

/** The export preview, keeping the last one on screen while the next renders. */
export function useExportPreview(
  meetingId: MeetingId,
  format: ExportFormat,
  sections: ExportSections,
) {
  return useQuery({
    ...previewQuery(meetingId, format, sections),
    placeholderData: (previous) => previous,
  });
}

/** What the save mutation takes. */
export interface SaveInput {
  format: ExportFormat;
  sections: ExportSections;
}

/** Saves the meeting through the save dialog. */
export function useSaveExport(meetingId: MeetingId) {
  return useMutation({
    mutationFn: ({ format, sections }: SaveInput) => exportApi.save(meetingId, format, sections),
  });
}

/** Copies the meeting to the clipboard. */
export function useCopyMeeting(meetingId: MeetingId) {
  return useMutation({ mutationFn: (kind: CopyKind) => exportApi.copy(meetingId, kind) });
}

/** Opens an email draft of the notes. */
export function useEmailMeeting(meetingId: MeetingId) {
  return useMutation({ mutationFn: () => exportApi.email(meetingId) });
}

/** Chooses the auto-export folder (the settings follow through `SettingsChanged`). */
export function useChooseExportFolder() {
  return useMutation({ mutationFn: exportApi.chooseFolder });
}

/** The account export's status, live. */
export function useFullExportStatus() {
  const client = useQueryClient();
  useIpcEvent(events.fullExportProgress, ({ status }) => {
    client.setQueryData<FullExportStatus>(exportKeys.fullStatus(), status);
  });
  return useQuery(fullExportStatusQuery());
}

/** Starts the account export (`includeAudio`: add the recordings). */
export function useStartFullExport() {
  return useMutation({
    mutationFn: (includeAudio: boolean) => exportApi.startFullExport(includeAudio),
  });
}

/** Cancels the account export. */
export function useCancelFullExport() {
  return useMutation({ mutationFn: exportApi.cancelFullExport });
}

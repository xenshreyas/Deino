import { Button } from '@/ui/button';

import { importCopy } from '../copy';
import { usePreviewImport, useStartImport } from '../queries';
import type { ImportPreview, ImportTarget } from '../types';

/** Props of {@link PreviewActions}. */
interface PreviewActionsProps {
  /** What to import; `null` until it is chosen (a folder). */
  target: ImportTarget | null;
  /** True while any import runs: only one runs at a time. */
  running: boolean;
}

/** Check first (the preview count), then Import (container). */
export function PreviewActions({ target, running }: PreviewActionsProps) {
  const preview = usePreviewImport();
  const start = useStartImport();
  const counted = preview.data;
  const canImport = counted !== undefined && counted.new > 0 && !running && target !== null;
  return (
    <div className="flex flex-col items-end gap-1">
      <div className="flex items-center gap-2">
        <Button
          variant="secondary"
          size="sm"
          disabled={target === null || preview.isPending || running}
          onClick={() => {
            if (target !== null) preview.mutate(target);
          }}
        >
          {preview.isPending ? importCopy.previewing : importCopy.preview}
        </Button>
        <Button
          size="sm"
          disabled={!canImport || start.isPending}
          onClick={() => {
            if (target === null) return;
            start.mutate(target, {
              onSuccess: () => {
                preview.reset();
              },
            });
          }}
        >
          {importCopy.import}
        </Button>
      </div>
      <PreviewLine counted={counted} error={preview.error?.message ?? start.error?.message} />
    </div>
  );
}

/** The count, or why it could not be made. */
function PreviewLine({
  counted,
  error,
}: {
  counted: ImportPreview | undefined;
  error: string | undefined;
}) {
  if (error !== undefined) {
    return (
      <p role="alert" className="max-w-72 text-right text-xs text-danger">
        {error}
      </p>
    );
  }
  if (counted === undefined) return null;
  return (
    <p role="status" className="text-xs text-fg-muted">
      {counted.new === 0
        ? importCopy.nothingNew
        : importCopy.previewResult(counted.new, counted.alreadyImported)}
    </p>
  );
}

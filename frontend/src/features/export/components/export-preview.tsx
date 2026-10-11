import { cn } from '@/ui/cn';
import { CircleAlert, ICON_PROPS } from '@/ui/icons';
import { SkeletonText } from '@/ui/skeleton';

import { exportCopy } from '../copy';
import { previewDocument, readPalette } from '../preview-document';
import type { ExportPreview as Preview } from '../types';

/** Props of {@link ExportPreview}. */
export interface ExportPreviewProps {
  preview: Preview | undefined;
  /** A newer preview is rendering; the old one stays, dimmed. */
  stale: boolean;
  failed: boolean;
}

/**
 * The file as it will be written: text formats as their text, documents
 * (PDF, DOCX) as a rendered page in a sandboxed frame that runs no script.
 */
export function ExportPreview({ preview, stale, failed }: ExportPreviewProps) {
  return (
    <div
      aria-label={exportCopy.previewLabel}
      aria-busy={stale || preview === undefined}
      className={cn(
        'relative min-h-0 flex-1 overflow-hidden rounded-lg border border-border bg-surface',
        'transition-opacity duration-(--dur-base) ease-out',
        stale && 'opacity-60',
      )}
    >
      <PreviewBody preview={preview} failed={failed} />
    </div>
  );
}

function PreviewBody({ preview, failed }: Omit<ExportPreviewProps, 'stale'>) {
  if (failed && preview === undefined) {
    return (
      <p role="alert" className="flex items-center gap-2 p-4 text-sm text-danger">
        <CircleAlert {...ICON_PROPS} />
        {exportCopy.previewError}
      </p>
    );
  }
  if (preview === undefined) {
    return (
      <div className="p-5">
        <span className="sr-only">{exportCopy.previewLoading}</span>
        <SkeletonText lines={9} />
      </div>
    );
  }
  if (preview.kind === 'html') {
    return (
      <iframe
        title={exportCopy.previewFrame}
        sandbox=""
        srcDoc={previewDocument(preview.content, readPalette(document.documentElement))}
        className="size-full border-0"
      />
    );
  }
  return (
    <pre className="size-full overflow-auto p-4 font-mono text-xs leading-5 whitespace-pre-wrap text-fg selection:bg-selection">
      {preview.content}
    </pre>
  );
}

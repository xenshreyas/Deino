/**
 * The notepad's editors, loaded on first use. TipTap and ProseMirror are
 * about a third of the app's JavaScript; keeping them out of the entry
 * chunk lets the shell paint first.
 */
import { forwardRef, lazy, Suspense } from 'react';

import { SkeletonText } from '@/ui/skeleton';

import type { NotepadProps } from './components/notepad';
import type { NotepadWindowProps } from './components/notepad-window';
import type { SummaryDocumentHandle, SummaryDocumentProps } from './components/summary-document';

const LazyNotepad = lazy(() =>
  import('./components/notepad').then((module) => ({ default: module.Notepad })),
);
const LazyNotepadWindow = lazy(() =>
  import('./components/notepad-window').then((module) => ({ default: module.NotepadWindow })),
);
const LazySummaryDocument = lazy(() =>
  import('./components/summary-document').then((module) => ({
    default: module.SummaryDocument,
  })),
);

/** A meeting's "My notes" (see `components/notepad.tsx`). */
export function Notepad(props: NotepadProps) {
  return (
    <Suspense fallback={<SkeletonText lines={3} />}>
      <LazyNotepad {...props} />
    </Suspense>
  );
}

/** A meeting's notepad as a whole window (see `components/notepad-window.tsx`). */
export function NotepadWindow(props: NotepadWindowProps) {
  return (
    <Suspense fallback={null}>
      <LazyNotepadWindow {...props} />
    </Suspense>
  );
}

/** The editable AI summary (see `components/summary-document.tsx`). */
export const SummaryDocument = forwardRef<SummaryDocumentHandle, SummaryDocumentProps>(
  function SummaryDocument(props, ref) {
    return (
      <Suspense fallback={<SkeletonText lines={5} />}>
        <LazySummaryDocument ref={ref} {...props} />
      </Suspense>
    );
  },
);

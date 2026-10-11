/**
 * The notepad's editors, loaded on first use. TipTap and ProseMirror are
 * about a third of the app's JavaScript; keeping them out of the entry
 * chunk lets the shell paint first.
 */
import { lazy, Suspense } from 'react';

import { SkeletonText } from '@/ui/skeleton';

import type { NotepadProps } from './components/notepad';
import type { NotepadWindowProps } from './components/notepad-window';

const LazyNotepad = lazy(() =>
  import('./components/notepad').then((module) => ({ default: module.Notepad })),
);
const LazyNotepadWindow = lazy(() =>
  import('./components/notepad-window').then((module) => ({ default: module.NotepadWindow })),
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

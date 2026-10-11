/**
 * The enhanced notes' editor, loaded on first use with the notepad's
 * schema: TipTap stays out of the entry chunk (as in `features/notepad`).
 */
import { lazy, Suspense } from 'react';

import { loadNoteExtensions } from '@/features/notepad';
import { SkeletonText } from '@/ui/skeleton';

import type { EnhancedEditorProps } from './components/enhanced-editor';

const LazyEditor = lazy(async () => {
  const [noteExtensions, module] = await Promise.all([
    loadNoteExtensions(),
    import('./components/enhanced-editor'),
  ]);
  function Loaded(props: Omit<EnhancedEditorProps, 'noteExtensions'>) {
    return <module.EnhancedEditor {...props} noteExtensions={noteExtensions} />;
  }
  return { default: Loaded };
});

/** The editor (see `components/enhanced-editor.tsx`). */
export function EnhancedEditor(props: Omit<EnhancedEditorProps, 'noteExtensions'>) {
  return (
    <Suspense fallback={<SkeletonText lines={6} />}>
      <LazyEditor {...props} />
    </Suspense>
  );
}

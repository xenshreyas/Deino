/**
 * Enhanced notes (NOT-02) and their citations (NOT-03): the user's notes
 * and the transcript, written up by the AI. User text is ink, AI text is
 * grey, every AI line opens the transcript moments behind it, and each
 * generation is a version.
 *
 * | Export | Where |
 * |---|---|
 * | {@link EnhancedNotes} | the meeting page's Notes tab, "Enhanced" |
 * | {@link EnhanceButton} | the meeting page's title bar (⌘⇧E), with the "Replace your edits?" dialog |
 * | {@link useEnhanceAction} | anything that regenerates (the template picker) |
 * | {@link CitedExcerpts} | the transcript excerpts a citation opens (NOT-03), reused by "Ask about this meeting" |
 *
 * The editor loads on first use (`lazy.tsx`). Other code imports from here only.
 */
export { enhanceApi } from './api';
export { CitedExcerpts } from './citations/citation-popover';
export { EnhanceButton, type EnhanceButtonProps } from './components/enhance-button';
export { EnhancedNotes, type EnhancedNotesProps } from './components/enhanced-notes';
export { notesCopy } from './copy';
export { useEnhanceAction } from './hooks/use-enhance-action';
export { enhanceKeys, enhancedNotesQuery, useEnhancedNotes, useIsEnhancing } from './queries';
export type { CitationRef, EnhancedNotesView, RegenerateReason, VersionInfo } from './types';

/**
 * The vocabulary feature (TRN-05): Settings → Vocabulary (terms with what
 * they sound like, and replacements) and a meeting's corrections with undo,
 * which the meeting's Details panel shows. Other code imports from here only.
 */
export { vocabularyApi } from './api';
export { MeetingCorrections, type MeetingCorrectionsProps } from './components/meeting-corrections';
export { VocabularySettings } from './components/vocabulary-settings';
export { vocabularyCopy } from './copy';
export { useCorrections, useVocabulary, vocabularyKeys, vocabularyQuery } from './queries';
export type { CorrectedSegment, TermInput, VocabKind, VocabTerm } from './types';

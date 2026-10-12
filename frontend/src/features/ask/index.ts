/**
 * "Ask about this meeting" (AI-02-lite, D13): single-meeting questions and
 * recipes (AI-04), answered from the meeting's redacted notes and
 * transcript, streamed, each statement citing the transcript moments behind
 * it (NOT-03's excerpts), kept as a short thread per meeting.
 *
 * | Export | Where |
 * |---|---|
 * | {@link AskPanel} | the meeting page's Chat panel (`features/meeting/slots.ts`) |
 * | {@link RecipePreview} | Recipes → Preview, handed to the page by the `/recipes` route |
 *
 * Cross-meeting chat (AI-02) stays parked: ⌘J and the entity and folder
 * chat slots keep their WIP state. Other code imports from here only.
 */
export { askApi } from './api';
export { AskPanel, type AskPanelProps } from './components/ask-panel';
export { RecipePreview, type RecipePreviewProps } from './components/recipe-preview';
export { askCopy } from './copy';
export { askKeys, useAskThread } from './queries';
export type { AnswerPart, AskMessage } from './types';

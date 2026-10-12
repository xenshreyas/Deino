/**
 * Comments (TRN-06): the user's private annotations on words of the
 * transcript or the notes. Select text → Comment; threads take replies and
 * can be resolved; the commented words are highlighted (the CSS Custom
 * Highlight API), and clicking them opens the thread. A transcript comment
 * is anchored by its line's ID and offsets plus the quoted words, so it
 * survives re-indexing and edits elsewhere, follows its words when they
 * move, and shows as "Detached" when they are gone.
 *
 * | Export | Where |
 * |---|---|
 * | {@link CommentLayer} | wraps the Transcript tab and the notes views |
 * | {@link CommentPopovers} | the meeting page, once: the composer and an opened thread |
 * | {@link CommentsPanel} | the meeting page's right panel |
 * | {@link startComment}, {@link transcriptSurface} | the transcript's Comment action (a whole line) |
 */
export { CommentLayer, type CommentLayerProps } from './components/comment-layer';
export { CommentPopovers } from './components/comment-popovers';
export { CommentsPanel, type CommentsPanelProps } from './components/comments-panel';
export type { SelectionAction } from './components/selection-menu';
export { commentsCopy } from './copy';
export { textOffset } from './model/dom-text';
export { bubbleOf, SEGMENT_ATTRIBUTE, transcriptSurface } from './model/surfaces';
export { commentKeys } from './queries';
export { screenRect, startComment } from './store';
export type { CommentThread } from './types';

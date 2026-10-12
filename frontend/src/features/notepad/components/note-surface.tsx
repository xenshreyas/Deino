/**
 * How a note looks: a status row (the quiet "Saved", plus whatever the page
 * puts on the right), then the document itself, scrolling, with room below
 * the last line so writing never happens at the window's bottom edge.
 * Clicking the blank space under the text puts the caret at the end. The
 * page's `footer` (insight answers, screen images) follows the document.
 */
import { EditorContent } from '@tiptap/react';
import type { ReactNode } from 'react';

import { MentionHoverZone } from '@/features/entities';

import type { NoteEditorHandle } from '../hooks/use-note-editor';
import { MentionMenu } from './mention-menu';
import { SavedIndicator } from './saved-indicator';
import { TemplateMenu } from './template-menu';

/** Props of {@link NoteSurface}. */
export interface NoteSurfaceProps {
  handle: NoteEditorHandle;
  /** Controls at the right of the status row (the Enhanced / My notes switch). */
  toolbar?: ReactNode;
  /** What follows the note inside its scroll. */
  footer?: ReactNode;
}

/** The editor and its status row. */
export function NoteSurface({ handle, toolbar, footer }: NoteSurfaceProps) {
  const { editor, mentions, templates, status } = handle;
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex h-9 shrink-0 items-center justify-between gap-3">
        <SavedIndicator status={status} />
        {toolbar}
      </div>
      <div
        className="min-h-0 flex-1 cursor-text overflow-y-auto pb-[30vh]"
        onMouseDown={(event) => {
          if (event.target !== event.currentTarget) return;
          event.preventDefault();
          editor.commands.focus('end');
        }}
      >
        <MentionHoverZone>
          <EditorContent editor={editor} />
        </MentionHoverZone>
        {footer === undefined ? null : <div className="cursor-auto">{footer}</div>}
      </div>
      <MentionMenu store={mentions} />
      <TemplateMenu store={templates} />
    </div>
  );
}

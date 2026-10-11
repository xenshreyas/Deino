/**
 * A line's text, editable in place (TRN-02): the bubble's own text becomes a
 * borderless field of the same size and type, focused with the caret at the
 * end. Enter saves, ⇧Enter breaks the line, Escape cancels, and leaving the
 * field saves. The row grows with the text (the list re-measures it).
 */
import { useQueryClient } from '@tanstack/react-query';
import { type KeyboardEvent, useCallback, useRef, useState } from 'react';

import { transcriptCopy } from '../copy';
import type { MeetingId, TranscriptLine } from '../types';
import { stopEditing } from './editing-store';
import { saveLineText } from './line-edits';

/** Props of {@link LineEditor}. */
export interface LineEditorProps {
  meetingId: MeetingId;
  line: TranscriptLine;
}

/** Fits the field's height to its text. */
function fit(field: HTMLTextAreaElement): void {
  field.style.height = '0px';
  field.style.height = `${String(field.scrollHeight)}px`;
}

/** Focuses the field once, caret at the end, sized to the text. */
function useAutofocus() {
  return useCallback((field: HTMLTextAreaElement | null) => {
    if (field === null) return;
    fit(field);
    field.focus();
    field.setSelectionRange(field.value.length, field.value.length);
  }, []);
}

/** The field. */
export function LineEditor({ meetingId, line }: LineEditorProps) {
  const client = useQueryClient();
  const [text, setText] = useState(line.text);
  const ref = useAutofocus();
  // Enter ends the edit and unmounts the field, which can also blur it.
  const done = useRef(false);
  const finish = (save: boolean) => {
    if (done.current) return;
    done.current = true;
    stopEditing(line.id);
    if (save && text !== line.text) {
      void saveLineText({ client, meetingId, segmentId: line.id }, text);
    }
  };
  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    event.stopPropagation();
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      finish(true);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      finish(false);
    }
  };
  return (
    <textarea
      ref={ref}
      value={text}
      rows={1}
      aria-label={transcriptCopy.editLine}
      onChange={(event) => {
        setText(event.target.value);
        fit(event.target);
      }}
      onKeyDown={onKeyDown}
      onBlur={() => {
        finish(true);
      }}
      className="block w-full resize-none overflow-hidden bg-transparent text-sm text-fg caret-accent outline-none"
    />
  );
}

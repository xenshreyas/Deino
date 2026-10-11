/**
 * One note editor: TipTap with Deino's schema, autosaving into its target
 * about a second after typing pauses, taking pasted and dropped images, and
 * turning on the shell's focus mode while the user types.
 */
import { type Editor, useEditor } from '@tiptap/react';
import { useState } from 'react';

import { useShellLayout } from '@/features/shell';
import type { Template } from '@/features/templates';
import { useMountEffect } from '@/hooks/useMountEffect';

import { imageUrl, notesApi, peopleApi } from '../api';
import { Autosaver } from '../autosave';
import { notepadCopy } from '../copy';
import { toNoteDoc } from '../doc-schema';
import { noteExtensions } from '../editor/extensions';
import { rankMentionCandidates, withNewPerson } from '../editor/mention-ranking';
import { createMentionStore, type MenuStore, type MentionStore } from '../editor/menu-store';
import type { TemplateCommandOptions } from '../editor/template-command';
import { imageFiles, insertFiles } from '../images';
import type { NoteTarget } from '../target';
import type { NoteDoc, SaveStatus } from '../types';
import { useTemplateCommand } from './use-template-command';

/** Options of {@link useNoteEditor}. */
export interface NoteEditorOptions {
  target: NoteTarget;
  initialDoc: NoteDoc | null;
  autoFocus: boolean;
}

/** What {@link useNoteEditor} hands to the view. */
export interface NoteEditorHandle {
  editor: Editor;
  mentions: MentionStore;
  /** The `/template` menu's store. */
  templates: MenuStore<Template>;
  status: SaveStatus;
  /** Saves pending edits now. */
  flush: () => Promise<void>;
}

/** Saves the editor's document; a blank draft that was never created stays uncreated. */
async function save(target: NoteTarget, editor: Editor): Promise<void> {
  if (editor.isEmpty && target.folder() === null) return;
  const { meetingId } = await target.resolve();
  await notesApi.save(meetingId, toNoteDoc(editor.getJSON()));
}

/** Keys that type a character turn focus mode on (ui.md, Granola's fade). */
function startsTyping(event: KeyboardEvent): boolean {
  return event.key.length === 1 && !event.metaKey && !event.ctrlKey && !event.altKey;
}

async function searchPeople(query: string) {
  return withNewPerson(rankMentionCandidates(await peopleApi.search(query), query), query);
}

/** The note schema for one editor: images resolve against the target's folder. */
function extensionsFor(
  target: NoteTarget,
  mentions: MentionStore,
  templates: TemplateCommandOptions,
) {
  return noteExtensions({
    placeholder: notepadCopy.placeholder,
    resolveImageSrc: (src) => {
      const folder = target.folder();
      return folder === null ? '' : imageUrl(folder, src);
    },
    mentions,
    searchPeople,
    templates,
  });
}

/** Saves pending edits when the page hides and when the editor unmounts. */
function useFlushOnLeave(saver: Autosaver<Editor>): void {
  useMountEffect(() => {
    const flush = () => void saver.flush();
    window.addEventListener('pagehide', flush);
    return () => {
      window.removeEventListener('pagehide', flush);
      flush();
    };
  });
}

/** Creates the editor and its autosave; flushes on unmount and page hide. */
export function useNoteEditor({
  target,
  initialDoc,
  autoFocus,
}: NoteEditorOptions): NoteEditorHandle {
  const [mentions] = useState(createMentionStore);
  const templates = useTemplateCommand(target);
  const [status, setStatus] = useState<SaveStatus>('idle');
  const [saver] = useState(
    () => new Autosaver<Editor>({ save: (editor) => save(target, editor), onStatus: setStatus }),
  );
  const editor = useEditor({
    extensions: extensionsFor(target, mentions, templates),
    content: initialDoc,
    autofocus: autoFocus ? 'end' : false,
    immediatelyRender: true,
    shouldRerenderOnTransaction: false,
    editorProps: editorProps(target),
    onUpdate: ({ editor: updated }) => {
      saver.schedule(updated);
    },
  });
  useFlushOnLeave(saver);
  return { editor, mentions, templates: templates.menu, status, flush: () => saver.flush() };
}

/** DOM handlers: images from paste and drop, focus mode while typing. */
function editorProps(target: NoteTarget) {
  return {
    attributes: {
      class: 'note-content',
      role: 'textbox',
      'aria-multiline': 'true',
      'aria-label': notepadCopy.editorLabel,
    },
    handleKeyDown: (_view: unknown, event: KeyboardEvent) => {
      if (startsTyping(event)) useShellLayout.getState().setFocusMode(true);
      return false;
    },
    handlePaste: (view: Parameters<typeof insertFiles>[0], event: ClipboardEvent) => {
      const files = imageFiles(event.clipboardData?.files);
      if (files.length === 0) return false;
      void insertFiles(view, target, files, null);
      return true;
    },
    handleDrop: (
      view: Parameters<typeof insertFiles>[0],
      event: DragEvent,
      _slice: unknown,
      moved: boolean,
    ) => {
      const files = moved ? [] : imageFiles(event.dataTransfer?.files);
      if (files.length === 0) return false;
      const pos = view.posAtCoords({ left: event.clientX, top: event.clientY })?.pos ?? null;
      void insertFiles(view, target, files, pos);
      return true;
    },
  };
}

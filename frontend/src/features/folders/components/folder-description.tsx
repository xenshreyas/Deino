/**
 * The folder's description, edited in place: a borderless field in the
 * body text that grows with its content and saves when it loses focus. It
 * tells chat (AI-02) what the folder is for.
 */
import { useState } from 'react';
import { toast } from 'sonner';

import { foldersCopy } from '../copy';
import { useSetFolderDescription } from '../mutations';
import type { Folder } from '../types';

/** The field; keyed by the saved text so a change from elsewhere resets it. */
export function FolderDescription({ folder }: { folder: Folder }) {
  const [draft, setDraft] = useState(folder.description);
  const save = useSetFolderDescription();
  const commit = () => {
    if (draft.trim() === folder.description) return;
    save.mutate(
      { id: folder.id, description: draft },
      { onError: (error) => toast.error(foldersCopy.actionFailed, { description: error.message }) },
    );
  };
  return (
    <textarea
      aria-label={foldersCopy.descriptionLabel}
      value={draft}
      rows={1}
      placeholder={foldersCopy.descriptionPlaceholder}
      onChange={(event) => {
        setDraft(event.target.value);
      }}
      onBlur={commit}
      className="field-sizing-content w-full resize-none rounded-sm bg-transparent text-sm leading-6 text-fg-muted outline-none placeholder:text-fg-subtle focus:text-fg"
    />
  );
}

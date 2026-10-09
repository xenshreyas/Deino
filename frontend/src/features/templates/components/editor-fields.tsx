import type { ReactNode } from 'react';

import { Input, Textarea } from '@/ui/input';

import { templatesCopy } from '../copy';
import type { Draft } from '../draft';

/** Props of {@link EditorFields}. */
export interface EditorFieldsProps {
  draft: Draft;
  readOnly: boolean;
  onChange: (patch: Partial<Draft>) => void;
}

/** Name, description and meeting context. */
export function EditorFields({ draft, readOnly, onChange }: EditorFieldsProps) {
  return (
    <div className="flex flex-col gap-4">
      <input
        aria-label={templatesCopy.name}
        placeholder={templatesCopy.namePlaceholder}
        value={draft.name}
        readOnly={readOnly}
        autoFocus={!readOnly && draft.name === ''}
        onChange={(event) => {
          onChange({ name: event.target.value });
        }}
        className="w-full rounded-sm bg-transparent text-xl font-semibold text-fg outline-none placeholder:text-fg-subtle focus-visible:shadow-[0_1px_0_var(--accent)]"
      />
      <Labelled label={templatesCopy.description} hint={templatesCopy.descriptionHint}>
        <Input
          value={draft.description}
          readOnly={readOnly}
          onChange={(event) => {
            onChange({ description: event.target.value });
          }}
        />
      </Labelled>
      <Labelled label={templatesCopy.meetingContext} hint={templatesCopy.meetingContextHint}>
        <Textarea
          value={draft.meetingContext}
          readOnly={readOnly}
          rows={3}
          onChange={(event) => {
            onChange({ meetingContext: event.target.value });
          }}
        />
      </Labelled>
    </div>
  );
}

/** A field with its label above and a hint below. */
function Labelled({ label, hint, children }: { label: string; hint: string; children: ReactNode }) {
  return (
    <label className="flex flex-col gap-1.5">
      <span className="text-sm text-fg">{label}</span>
      {children}
      <span className="text-xs text-fg-muted">{hint}</span>
    </label>
  );
}

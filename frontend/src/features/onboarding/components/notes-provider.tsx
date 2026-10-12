/**
 * Who writes the notes: Claude or OpenAI with the user's key, or the
 * built-in model on this Mac. Onboarding's AI step and Settings → Notes →
 * AI (UX-11) show the same three choices.
 */
import { useState } from 'react';

import { BuiltInModelManager } from '@/components/BuiltInModelManager';
import { type Settings, useSaveModelSelection } from '@/features/settings';
import { SegmentedControl } from '@/ui/segmented-control';

import { initialChoice, isKeyed, type NotesChoice } from '../ai-choice';
import { aiCopy } from '../copy';
import { ProviderKeyForm } from './provider-key-form';

/** Props of {@link NotesProvider}. */
export interface NotesProviderProps {
  ai: Settings['ai'];
  /** Show the "Notes" heading (onboarding); Settings has its own. */
  heading?: boolean;
}

/** Who writes the notes, starting from the saved provider. */
export function NotesProvider({ ai, heading = true }: NotesProviderProps) {
  const [choice, setChoice] = useState<NotesChoice>(() => initialChoice(ai));
  return (
    <section aria-label={aiCopy.notesTitle} className="flex flex-col gap-4">
      {heading ? (
        <h2 className="text-sm font-semibold text-fg-muted">{aiCopy.notesTitle}</h2>
      ) : null}
      <SegmentedControl
        label={aiCopy.notesTitle}
        value={choice}
        onValueChange={setChoice}
        segments={[
          { value: 'claude', label: aiCopy.choices.claude },
          { value: 'openai', label: aiCopy.choices.openai },
          { value: 'local', label: aiCopy.choices.local },
        ]}
      />
      {isKeyed(choice) ? (
        <ProviderKeyForm key={choice} choice={choice} />
      ) : (
        <LocalNotesModel selected={ai.providerModels['builtin-ai'] ?? ''} />
      )}
    </section>
  );
}

/** The built-in model, downloaded and chosen with the existing manager. */
function LocalNotesModel({ selected }: { selected: string }) {
  const save = useSaveModelSelection();
  return (
    <div className="flex flex-col gap-3">
      <p className="text-sm text-fg-muted">{aiCopy.localWhy}</p>
      <BuiltInModelManager
        selectedModel={selected}
        onModelSelect={(model) => {
          save.mutate({ provider: 'builtin-ai', model });
        }}
      />
    </div>
  );
}

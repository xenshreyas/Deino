import { useState } from 'react';

import { BuiltInModelManager } from '@/components/BuiltInModelManager';
import { type Settings, useSaveModelSelection, useSettings } from '@/features/settings';
import { ModelLicenceNotice } from '@/features/transcription';
import { SegmentedControl } from '@/ui/segmented-control';

import { initialChoice, isKeyed, type NotesChoice } from '../../ai-choice';
import { aiCopy } from '../../copy';
import { useEmbeddingModel, useTranscriptionModel } from '../../hooks/use-local-models';
import { ModelDownloadRow } from '../model-download-row';
import { ProviderKeyForm } from '../provider-key-form';
import { StepFrame, type StepProps } from '../step-frame';

/**
 * Step 3: the local models download right away, while the user picks who
 * writes the notes: Claude or OpenAI with a key, or the built-in model.
 */
export function AiSetupStep({ dispatch }: StepProps) {
  const settings = useSettings().data;
  return (
    <StepFrame
      step="ai_setup"
      title={aiCopy.title}
      description={aiCopy.description}
      dispatch={dispatch}
    >
      <LocalModels />
      {settings === undefined ? null : <NotesProvider ai={settings.ai} />}
    </StepFrame>
  );
}

/** Parakeet and the embedding model, downloading as soon as they show. */
function LocalModels() {
  const transcription = useTranscriptionModel();
  const embedding = useEmbeddingModel();
  return (
    <section aria-label={aiCopy.models} className="flex flex-col gap-2">
      <h2 className="text-sm font-semibold text-fg-muted">{aiCopy.models}</h2>
      <ul className="flex flex-col divide-y divide-border border-y border-border">
        <ModelDownloadRow
          name={aiCopy.transcription}
          why={aiCopy.transcriptionWhy}
          model={transcription}
        />
        <ModelDownloadRow name={aiCopy.embedding} why={aiCopy.embeddingWhy} model={embedding} />
      </ul>
      <ModelLicenceNotice />
    </section>
  );
}

/** Who writes the notes, starting from the saved provider. */
function NotesProvider({ ai }: { ai: Settings['ai'] }) {
  const [choice, setChoice] = useState<NotesChoice>(() => initialChoice(ai));
  return (
    <section aria-label={aiCopy.notesTitle} className="flex flex-col gap-4">
      <h2 className="text-sm font-semibold text-fg-muted">{aiCopy.notesTitle}</h2>
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

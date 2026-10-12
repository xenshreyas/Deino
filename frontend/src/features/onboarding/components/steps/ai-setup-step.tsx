import { useSettings } from '@/features/settings';
import { ModelLicenceNotice } from '@/features/transcription';

import { aiCopy } from '../../copy';
import { useEmbeddingModel, useTranscriptionModel } from '../../hooks/use-local-models';
import { ModelDownloadRow } from '../model-download-row';
import { NotesProvider } from '../notes-provider';
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

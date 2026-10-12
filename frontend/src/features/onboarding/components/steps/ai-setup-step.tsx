import { useSettings } from '@/features/settings';
import { ModelLicenceNotice } from '@/features/transcription';

import { aiCopy } from '../../copy';
import { useEmbeddingModel, useTranscriptionModel } from '../../hooks/use-local-models';
import { AutoStart, TranscriptionStatus } from '../model-download-row';
import { NotesProvider } from '../notes-provider';
import { StepFrame, type StepProps } from '../step-frame';

/**
 * Step 3: transcription gets ready right away (one plain line, UX-23), the
 * search model downloads without a row (UX-21), and the user picks who
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

/** Transcription's line; the search model starts quietly beside it. */
function LocalModels() {
  const transcription = useTranscriptionModel();
  const embedding = useEmbeddingModel();
  return (
    <section aria-label={aiCopy.transcription} className="flex flex-col gap-2">
      <h2 className="text-sm font-semibold text-fg-muted">{aiCopy.transcription}</h2>
      <TranscriptionStatus model={transcription} />
      <AutoStart model={embedding} />
      <ModelLicenceNotice />
    </section>
  );
}

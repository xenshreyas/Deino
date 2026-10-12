/**
 * Settings → Transcription's engine picker. The select and the model manager
 * show `settings.transcription.engine` and `.model` exactly as saved, the
 * values the recorder reads, and every choice is saved at once. There is no
 * local copy and no default standing in for the saved value, so the page and
 * the recorder cannot disagree.
 */
import { ParakeetModelManager } from '@/components/ParakeetModelManager';
import { ModelManager as WhisperModelManager } from '@/components/WhisperModelManager';
import { ParakeetUnifiedManager } from '@/features/transcription';
import { Label } from '@/ui/controls';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from '@/ui/select';

import { settingsCopy } from '../copy';
import { usePatchSettings, useSettings } from '../queries';
import type { TranscriptionEngine, TranscriptionSettings } from '../types';

const copy = settingsCopy.engine;

/** Whether `value` (a select value) names an engine. */
export function isEngine(value: string): value is TranscriptionEngine {
  return Object.hasOwn(copy.names, value);
}

/** The engine picker over the saved settings. */
export function EngineSettings() {
  const settings = useSettings();
  const { mutate: patch } = usePatchSettings();
  if (settings.data === undefined) {
    const message = settings.isError ? copy.loadFailed : copy.loading;
    return <p className="text-sm text-fg-muted">{message}</p>;
  }
  return (
    <EnginePicker
      transcription={settings.data.transcription}
      onChoose={(engine, model) => {
        patch({ transcription: { engine, model } });
      }}
    />
  );
}

interface EnginePickerProps {
  /** The saved transcription settings. */
  transcription: TranscriptionSettings;
  /** Saves `engine` with `model` (empty: the engine's recommended model). */
  onChoose: (engine: TranscriptionEngine, model: string) => void;
}

/** The engine select, then the saved engine's model manager. */
export function EnginePicker({ transcription, onChoose }: EnginePickerProps) {
  const { engine } = transcription;
  return (
    <div className="flex flex-col gap-4 pb-6">
      <div className="flex flex-col gap-1">
        <Label htmlFor="transcription-engine">{copy.label}</Label>
        <Select
          value={engine}
          onValueChange={(value) => {
            if (isEngine(value) && value !== engine) onChoose(value, '');
          }}
        >
          <SelectTrigger id="transcription-engine">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="parakeet_unified">{copy.names.parakeet_unified}</SelectItem>
            <SelectGroup>
              <SelectLabel>{copy.advanced}</SelectLabel>
              <SelectItem value="parakeet">{copy.names.parakeet}</SelectItem>
              <SelectItem value="whisper">{copy.names.whisper}</SelectItem>
            </SelectGroup>
          </SelectContent>
        </Select>
      </div>
      <ModelManager transcription={transcription} onChoose={onChoose} />
    </div>
  );
}

/** The model manager of the saved engine, its saved model selected. */
function ModelManager({ transcription, onChoose }: EnginePickerProps) {
  const { engine, model } = transcription;
  const choose = (next: string) => {
    onChoose(engine, next);
  };
  switch (engine) {
    case 'parakeet_unified':
      return <ParakeetUnifiedManager selectedModel={model} onModelSelect={choose} />;
    case 'parakeet':
      return <ParakeetModelManager selectedModel={model} onModelSelect={choose} />;
    case 'whisper':
      return <WhisperModelManager selectedModel={model} onModelSelect={choose} />;
  }
}

import { useState, useEffect } from 'react';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from './ui/select';
import { Label } from './ui/label';
import { ModelManager } from './WhisperModelManager';
import { ParakeetModelManager } from './ParakeetModelManager';

/** Local transcription engines. Transcription never leaves the Mac. */
export type TranscriptProvider = 'localWhisper' | 'parakeet';

export interface TranscriptModelProps {
    provider: TranscriptProvider;
    model: string;
}

export interface TranscriptSettingsProps {
    transcriptModelConfig: TranscriptModelProps;
    setTranscriptModelConfig: (config: TranscriptModelProps) => void;
    onModelSelect?: () => void;
}

/** Lets the user pick a local transcription engine and its model. */
export function TranscriptSettings({ transcriptModelConfig, setTranscriptModelConfig, onModelSelect }: TranscriptSettingsProps) {
    const [uiProvider, setUiProvider] = useState<TranscriptProvider>(transcriptModelConfig.provider);

    // Sync uiProvider when backend config changes (e.g., after model selection or initial load)
    useEffect(() => {
        setUiProvider(transcriptModelConfig.provider);
    }, [transcriptModelConfig.provider]);

    const selectModel = (provider: TranscriptProvider, model: string) => {
        setTranscriptModelConfig({ ...transcriptModelConfig, provider, model });
        onModelSelect?.();
    };

    return (
        <div className="space-y-4 pb-6">
            <div>
                <Label className="block text-sm font-medium text-gray-700 mb-1">
                    Transcript Model
                </Label>
                <div className="flex space-x-2 mx-1">
                    <Select
                        value={uiProvider}
                        onValueChange={(value) => setUiProvider(value as TranscriptProvider)}
                    >
                        <SelectTrigger className='focus:ring-1 focus:ring-blue-500 focus:border-blue-500'>
                            <SelectValue placeholder="Select provider" />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem value="parakeet">⚡ Parakeet (Recommended - Real-time / Accurate)</SelectItem>
                            <SelectItem value="localWhisper">🏠 Local Whisper (High Accuracy)</SelectItem>
                        </SelectContent>
                    </Select>
                </div>
            </div>

            {uiProvider === 'localWhisper' && (
                <div className="mt-6">
                    <ModelManager
                        selectedModel={transcriptModelConfig.provider === 'localWhisper' ? transcriptModelConfig.model : undefined}
                        onModelSelect={(model) => selectModel('localWhisper', model)}
                        autoSave={true}
                    />
                </div>
            )}

            {uiProvider === 'parakeet' && (
                <div className="mt-6">
                    <ParakeetModelManager
                        selectedModel={transcriptModelConfig.provider === 'parakeet' ? transcriptModelConfig.model : undefined}
                        onModelSelect={(model) => selectModel('parakeet', model)}
                        autoSave={true}
                    />
                </div>
            )}
        </div>
    );
}

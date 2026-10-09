import { useState, useEffect } from 'react';
import { Select, SelectContent, SelectGroup, SelectItem, SelectLabel, SelectTrigger, SelectValue } from '@/ui/select';
import { Label } from '@/ui/controls';
import { ParakeetUnifiedManager } from '@/features/transcription';
import { ModelManager } from './WhisperModelManager';
import { ParakeetModelManager } from './ParakeetModelManager';

/**
 * Local transcription engines. Transcription never leaves the Mac.
 * Parakeet Unified is the default (CAP-01); the others are "Advanced".
 */
export type TranscriptProvider = 'parakeetUnified' | 'localWhisper' | 'parakeet';

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
                <Label className="block text-sm font-medium text-fg mb-1">
                    Transcript Model
                </Label>
                <div className="flex space-x-2 mx-1">
                    <Select
                        value={uiProvider}
                        onValueChange={(value) => setUiProvider(value as TranscriptProvider)}
                    >
                        <SelectTrigger className='focus:ring-1 focus:ring-focus-ring focus:border-accent'>
                            <SelectValue placeholder="Select provider" />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem value="parakeetUnified">Parakeet Unified EN (Recommended, live)</SelectItem>
                            <SelectGroup>
                                <SelectLabel>Advanced</SelectLabel>
                                <SelectItem value="parakeet">Parakeet TDT v3 (ONNX)</SelectItem>
                                <SelectItem value="localWhisper">Whisper</SelectItem>
                            </SelectGroup>
                        </SelectContent>
                    </Select>
                </div>
            </div>

            {uiProvider === 'parakeetUnified' && (
                <div className="mt-6">
                    <ParakeetUnifiedManager
                        selectedModel={transcriptModelConfig.provider === 'parakeetUnified' ? transcriptModelConfig.model : undefined}
                        onModelSelect={(model) => selectModel('parakeetUnified', model)}
                    />
                </div>
            )}

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

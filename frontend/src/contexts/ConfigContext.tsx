/**
 * App-wide configuration for the legacy components, backed by the typed
 * settings (FND-06).
 *
 * Every value here is derived from `useSettings()` and every setter writes
 * a settings patch, so there is one source of truth and nothing to sync.
 * The one piece of local state is the model picker's unsaved draft. API
 * keys are not here: the UI can only ask whether one is saved
 * (`useSecretStatus` from `@/features/settings`).
 *
 * New code should use `@/features/settings` directly; this context shrinks
 * as FND-09/FND-10 rebuild the screens that read it.
 */
import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from 'react';
import { useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';

import type { ModelConfig } from '@/components/ModelSettingsModal';
import { legacyOllamaModels } from '@/features/llm';
import type { SelectedDevices } from '@/components/DeviceSelection';
import {
  type AiSettings,
  type ModelSelection,
  type NotificationSettings,
  type Settings,
  type SettingsPatch,
  type TranscriptionSettings,
  usePatchSettings,
  useSaveModelSelection,
  useSettings,
} from '@/features/settings';
import { PARAKEET_UNIFIED_PROVIDER, providerOf, type TranscriptProviderName } from '@/features/transcription';

export type { NotificationSettings } from '@/features/settings';

/** The saved engine and model under their legacy provider names, for the
 * retranscribe dialog. Settings → Transcription reads the settings itself. */
export interface TranscriptModelProps {
  provider: TranscriptProviderName;
  model: string;
}

export interface OllamaModel {
  name: string;
  id: string;
  size: string;
  modified: string;
}

export interface StorageLocations {
  database: string;
  models: string;
  recordings: string;
}

interface ConfigContextType {
  /** The summary model: the picker's unsaved draft, else the saved choice. */
  modelConfig: ModelConfig;
  /** Edits the draft; nothing is saved until `saveModelConfig`. */
  setModelConfig: (config: ModelConfig | ((prev: ModelConfig) => ModelConfig)) => void;
  /** Saves a model choice (and a typed key, to the Keychain) and drops the draft. */
  saveModelConfig: (config: ModelConfig) => Promise<void>;
  isModelConfigLoading: boolean;

  transcriptModelConfig: TranscriptModelProps;

  selectedDevices: SelectedDevices;
  setSelectedDevices: (devices: SelectedDevices) => void;

  showConfidenceIndicator: boolean;
  toggleConfidenceIndicator: (checked: boolean) => void;

  models: OllamaModel[];
  modelOptions: Record<ModelConfig['provider'], string[]>;
  error: string;

  isAutoSummary: boolean;
  toggleIsAutoSummary: (checked: boolean) => void;

  notificationSettings: NotificationSettings | null;
  updateNotificationSettings: (settings: NotificationSettings) => Promise<void>;

  storageLocations: StorageLocations | null;
  isLoadingPreferences: boolean;
  loadPreferences: () => Promise<void>;
}

const ConfigContext = createContext<ConfigContextType | undefined>(undefined);

/** The legacy model config for the saved AI settings. Keys are never included. */
export function toModelConfig(ai: AiSettings): ModelConfig {
  const custom = ai.customOpenai;
  return {
    provider: ai.provider,
    model: ai.model,
    apiKey: null,
    ollamaEndpoint: ai.ollamaEndpoint,
    customOpenAIEndpoint: custom?.endpoint ?? null,
    customOpenAIModel: custom?.model ?? null,
    customOpenAIApiKey: null,
    maxTokens: custom?.maxTokens ?? null,
    temperature: custom?.temperature ?? null,
    topP: custom?.topP ?? null,
  };
}

/** What saving `config` writes: the settings fields and any typed key. */
export function toModelSelection(config: ModelConfig): ModelSelection {
  const isCustom = config.provider === 'custom-openai';
  const endpoint = config.customOpenAIEndpoint?.trim() ?? '';
  return {
    provider: config.provider,
    model: config.model,
    ollamaEndpoint: config.ollamaEndpoint?.trim() || null,
    customOpenai: isCustom && endpoint !== ''
      ? {
          endpoint,
          model: config.customOpenAIModel?.trim() || config.model,
          maxTokens: config.maxTokens ?? null,
          temperature: config.temperature ?? null,
          topP: config.topP ?? null,
        }
      : null,
    typedKey: isCustom ? config.customOpenAIApiKey ?? null : config.apiKey ?? null,
  };
}

/** The legacy transcript config for the saved transcription settings. */
function toTranscriptConfig(transcription: TranscriptionSettings): TranscriptModelProps {
  return {
    provider: providerOf(transcription.engine),
    model: transcription.model,
  };
}

const FALLBACK_MODEL_CONFIG: ModelConfig = { provider: 'builtin-ai', model: '' };
const FALLBACK_TRANSCRIPT_CONFIG: TranscriptModelProps = {
  provider: PARAKEET_UNIFIED_PROVIDER,
  model: '',
};

export function ConfigProvider({ children }: { children: ReactNode }) {
  const { data: settings, isPending } = useSettings();
  const { mutateAsync: patch } = usePatchSettings();
  const { mutateAsync: saveSelection } = useSaveModelSelection();
  const [draft, setDraft] = useState<ModelConfig | null>(null);
  const preferences = useStoragePreferences();

  const savedModelConfig = settings ? toModelConfig(settings.ai) : FALLBACK_MODEL_CONFIG;
  const modelConfig = draft ?? savedModelConfig;
  const ollama = useOllamaModels(settings?.ai.ollamaEndpoint ?? null);

  const persist = useCallback((change: SettingsPatch) => {
    patch(change).catch((error: unknown) => {
      console.error('[ConfigContext] Failed to save settings:', error);
    });
  }, [patch]);

  const setModelConfig = useCallback(
    (config: ModelConfig | ((prev: ModelConfig) => ModelConfig)) => {
      setDraft((current) => {
        const base = current ?? savedModelConfig;
        return typeof config === 'function' ? config(base) : config;
      });
    },
    [savedModelConfig],
  );

  const saveModelConfig = useCallback(async (config: ModelConfig) => {
    await saveSelection(toModelSelection(config));
    setDraft(null);
  }, [saveSelection]);

  const value = useMemo<ConfigContextType>(() => ({
    modelConfig,
    setModelConfig,
    saveModelConfig,
    isModelConfigLoading: isPending,
    ...derivedSettings(settings, persist),
    models: ollama.models,
    modelOptions: modelOptionsFor(ollama.models),
    error: ollama.error,
    ...preferences,
  }), [modelConfig, setModelConfig, saveModelConfig, isPending, settings, persist, ollama, preferences]);

  return <ConfigContext.Provider value={value}>{children}</ConfigContext.Provider>;
}

/** The settings-backed values and their setters. */
function derivedSettings(
  settings: Settings | undefined,
  persist: (change: SettingsPatch) => void,
) {
  return {
    transcriptModelConfig: settings ? toTranscriptConfig(settings.transcription) : FALLBACK_TRANSCRIPT_CONFIG,
    selectedDevices: {
      micDevice: settings?.audio.preferredMicDevice ?? null,
      systemDevice: settings?.audio.preferredSystemDevice ?? null,
    },
    setSelectedDevices: (devices: SelectedDevices) => {
      persist({
        audio: { preferredMicDevice: devices.micDevice, preferredSystemDevice: devices.systemDevice },
      });
    },
    showConfidenceIndicator: settings?.ui.showConfidenceIndicator ?? true,
    toggleConfidenceIndicator: (checked: boolean) => {
      persist({ ui: { showConfidenceIndicator: checked } });
    },
    isAutoSummary: settings?.ai.autoSummary ?? false,
    toggleIsAutoSummary: (checked: boolean) => {
      persist({ ai: { autoSummary: checked } });
    },
    notificationSettings: settings?.notifications ?? null,
    updateNotificationSettings: async (notifications: NotificationSettings) => {
      persist({ notifications });
    },
  };
}

/** The models of the configured Ollama server; empty while it is unreachable. */
function useOllamaModels(endpoint: string | null): { models: OllamaModel[]; error: string } {
  const query = useQuery({
    queryKey: ['ollama-models', endpoint],
    queryFn: () => legacyOllamaModels(endpoint),
    retry: false,
  });
  const error = query.error ? query.error.message || 'Failed to load Ollama models' : '';
  return useMemo(() => ({ models: query.data ?? [], error }), [query.data, error]);
}

function modelOptionsFor(models: OllamaModel[]): Record<ModelConfig['provider'], string[]> {
  return {
    ollama: models.map((model) => model.name),
    claude: ['claude-3-5-sonnet-latest'],
    groq: ['llama-3.3-70b-versatile'],
    openrouter: [],
    openai: ['gpt-4', 'gpt-4-turbo', 'gpt-3.5-turbo'],
    'builtin-ai': [],
    'custom-openai': [],
  };
}

/** The storage folder paths, loaded on first request. */
function useStoragePreferences() {
  const [storageLocations, setStorageLocations] = useState<StorageLocations | null>(null);
  const [isLoadingPreferences, setIsLoadingPreferences] = useState(false);

  const loadPreferences = useCallback(async () => {
    if (storageLocations !== null || isLoadingPreferences) return;
    setIsLoadingPreferences(true);
    try {
      const [database, models, recordings] = await Promise.all([
        invoke<string>('get_database_directory'),
        invoke<string>('whisper_get_models_directory'),
        invoke<string>('get_default_recordings_folder_path'),
      ]);
      setStorageLocations({ database, models, recordings });
    } catch (error) {
      console.error('[ConfigContext] Failed to load storage locations:', error);
    } finally {
      setIsLoadingPreferences(false);
    }
  }, [storageLocations, isLoadingPreferences]);

  return useMemo(
    () => ({ storageLocations, isLoadingPreferences, loadPreferences }),
    [storageLocations, isLoadingPreferences, loadPreferences],
  );
}

export function useConfig() {
  const context = useContext(ConfigContext);
  if (context === undefined) {
    throw new Error('useConfig must be used within a ConfigProvider');
  }
  return context;
}

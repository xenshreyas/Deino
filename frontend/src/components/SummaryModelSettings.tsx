import { toast } from 'sonner';
import { ModelConfig, ModelSettingsModal } from '@/components/ModelSettingsModal';
import { useConfig } from '@/contexts/ConfigContext';

/**
 * Settings → Notes → AI → Other providers (UX-11): the inherited model
 * panel, for Groq, OpenRouter, Ollama and custom endpoints. The Auto
 * Summary switch is Settings → Advanced → Notes after meetings.
 *
 * The model comes from the typed settings through `ConfigContext` (FND-06);
 * a saved change reaches every other screen through `SettingsChanged`.
 */
export function OtherProviders() {
  const { modelConfig, setModelConfig, saveModelConfig } = useConfig();

  const handleSaveModelConfig = async (config: ModelConfig) => {
    try {
      await saveModelConfig(config); // UX-31: saved silently
    } catch (error) {
      console.error('Error saving model config:', error);
      toast.error('Failed to save model settings', {
        description: error instanceof Error ? error.message : undefined,
      });
    }
  };

  return (
    <ModelSettingsModal
      modelConfig={modelConfig}
      setModelConfig={setModelConfig}
      onSave={handleSaveModelConfig}
      skipInitialFetch={true}
    />
  );
}

import { toast } from 'sonner';
import { ModelConfig, ModelSettingsModal } from '@/components/ModelSettingsModal';
import { Switch } from '@/ui/controls';
import { useConfig } from '@/contexts/ConfigContext';

/**
 * The Summary tab of Settings: auto-summary and the summary model.
 *
 * Both come from the typed settings through `ConfigContext` (FND-06); a
 * saved change reaches every other screen through `SettingsChanged`.
 */
export function SummaryModelSettings() {
  const { modelConfig, setModelConfig, saveModelConfig, isAutoSummary, toggleIsAutoSummary } = useConfig();

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
    <div className='flex flex-col gap-4'>
      <div className="bg-surface rounded-lg border border-border p-6 shadow-xs">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-lg font-semibold text-fg mb-2">Auto Summary</h3>
            <p className="text-sm text-fg-muted">Auto Generating summary after meeting completion(Stopping)</p>
          </div>
          <Switch checked={isAutoSummary} onCheckedChange={toggleIsAutoSummary} />
        </div>
      </div>

      <div className="bg-surface rounded-lg border border-border p-6 shadow-xs">
        <h3 className="text-lg font-semibold mb-4">Summary Model Configuration</h3>
        <p className="text-sm text-fg-muted mb-6">
          Configure the AI model used for generating meeting summaries.
        </p>

        <ModelSettingsModal
          modelConfig={modelConfig}
          setModelConfig={setModelConfig}
          onSave={handleSaveModelConfig}
          skipInitialFetch={true}
        />
      </div>
    </div>
  );
}

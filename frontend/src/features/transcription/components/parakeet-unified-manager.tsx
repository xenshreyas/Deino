import { copy } from '../copy';
import { activeModel } from '../models';
import {
  useCancelDownload,
  useDeleteModel,
  useDownloadModel,
  useModelEvents,
  useModels,
} from '../queries';
import type { ModelFileId, ModelFileView } from '../types';
import { ModelRow, type ModelRowActions } from './model-row';

interface ParakeetUnifiedManagerProps {
  /** The configured file; empty means "the recommended one". */
  selectedModel: string | undefined;
  /** Makes `id` the file recordings use. */
  onModelSelect: (id: ModelFileId) => void;
}

/**
 * The model manager for Parakeet Unified: every quant with its size and
 * state, and Download, Resume, Cancel, Delete and Use. A finished download
 * becomes the file in use.
 */
export function ParakeetUnifiedManager({
  selectedModel,
  onModelSelect,
}: ParakeetUnifiedManagerProps) {
  useModelEvents();
  const models = useModels();
  const actions = useRowActions(onModelSelect);
  const error = actions.error?.message;

  if (models.isPending) return <p className="text-sm text-fg-muted">{copy.loading}</p>;
  if (models.isError) return <p className="text-sm text-danger">{copy.loadFailed}</p>;

  const inUse = activeModel(models.data, selectedModel);
  return (
    <section className="flex flex-col gap-2">
      <h3 className="text-sm font-semibold">{copy.title}</h3>
      <p className="text-xs text-fg-muted">{copy.description}</p>
      {error === undefined ? null : <p className="text-xs text-danger">{error}</p>}
      <ul className="divide-y divide-border">
        {models.data.map((model) => (
          <ModelRow
            key={model.id}
            model={model}
            inUse={model.id === inUse?.id}
            actions={actions.for(model)}
          />
        ))}
      </ul>
      <p className="text-xs text-fg-muted">{copy.licence}</p>
    </section>
  );
}

/** The row actions, wired to the mutations, plus the latest error. */
function useRowActions(onModelSelect: (id: ModelFileId) => void) {
  const download = useDownloadModel();
  const cancel = useCancelDownload();
  const remove = useDeleteModel();
  return {
    error: download.error ?? remove.error,
    for: (model: ModelFileView): ModelRowActions => ({
      onDownload: () => {
        download.mutate(model.id, {
          onSuccess: () => {
            onModelSelect(model.id);
          },
        });
      },
      onCancel: () => {
        cancel.mutate(model.id);
      },
      onDelete: () => {
        remove.mutate(model.id);
      },
      onUse: () => {
        onModelSelect(model.id);
      },
    }),
  };
}

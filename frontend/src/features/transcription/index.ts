/**
 * Transcription (CAP-01): the Parakeet Unified EN 0.6B model manager. The
 * live transcript is the `recording` feature's (CAP-02).
 *
 * | File | Role |
 * |---|---|
 * | `types.ts` | Types, re-exported from the generated bindings |
 * | `api.ts` | The only IPC calls, plus event subscriptions for non-React code |
 * | `models.ts` | Pure rules: the file in use, readiness, sizes |
 * | `queries.ts` | The model list query, its mutations and event sync |
 * | `components/` | The model manager (container + rows) |
 *
 * Engine selection is still the legacy transcript config: the provider
 * {@link PARAKEET_UNIFIED_PROVIDER} with the chosen file's ID as its model.
 */
export { PARAKEET_UNIFIED_PROVIDER, transcriptionApi } from './api';
export { ModelLicenceNotice } from './components/model-licence-notice';
export { ParakeetUnifiedManager } from './components/parakeet-unified-manager';
export { activeModel, fileIdOf, isReady } from './models';
export { providerOf, type TranscriptProviderName } from './engine';
export {
  modelsQuery,
  transcriptionKeys,
  useDownloadModel,
  useModelEvents,
  useModels,
} from './queries';
export type { DownloadProgress, ModelFileId, ModelFileView, Quant } from './types';

/**
 * Transcription (CAP-01): the Parakeet Unified EN 0.6B model manager and the
 * live transcript line.
 *
 * | File | Role |
 * |---|---|
 * | `types.ts` | Types, re-exported from the generated bindings |
 * | `api.ts` | The only IPC calls, plus event subscriptions for non-React code |
 * | `models.ts` | Pure rules: the file in use, readiness, sizes |
 * | `queries.ts` | The model list query, its mutations and event sync |
 * | `hooks/` | The live transcript subscription |
 * | `components/` | The model manager (container + rows) and the live line |
 *
 * Engine selection is still the legacy transcript config: the provider
 * {@link PARAKEET_UNIFIED_PROVIDER} with the chosen file's ID as its model.
 */
export { PARAKEET_UNIFIED_PROVIDER, transcriptionApi } from './api';
export { LiveTranscriptLine } from './components/live-transcript-line';
export { ModelLicenceNotice } from './components/model-licence-notice';
export {
  type DefaultModelProgress,
  downloadDefaultModel,
  isAnyModelDownloading,
  isDefaultModelReady,
  onDefaultModelProgress,
} from './default-model';
export { ParakeetUnifiedManager } from './components/parakeet-unified-manager';
export { activeModel, isReady } from './models';
export { engineOf, providerOf, type TranscriptProviderName } from './engine';
export { modelsQuery, transcriptionKeys, useModels } from './queries';
export type { DownloadProgress, LiveTranscript, ModelFileId, ModelFileView, Quant } from './types';

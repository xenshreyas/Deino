/**
 * Beta Features Type System
 *
 * This file defines the scalable architecture for managing beta features.
 *
 * ## Adding a New Beta Feature
 * 1. Add property to BetaFeatures interface
 * 2. Add default value in DEFAULT_BETA_FEATURES
 * 3. Add UI strings in BETA_FEATURE_NAMES and BETA_FEATURE_DESCRIPTIONS
 * 4. Use in components: `betaFeatures.yourFeatureName`
 *
 * The values are stored in the typed settings (`Settings.beta`, FND-06); a
 * new flag also needs its field in `crates/deino-core/src/settings/ui.rs`.
 *
 * ## Graduating a Feature to Stable
 * 1. Remove property from BetaFeatures interface
 * 2. TypeScript will error at all usage sites
 * 3. Remove conditional checks - feature is now always-on
 */

export interface BetaFeatures {
  /**
   * Retranscribe existing meetings with a different transcription model.
   * (Importing graduated out of beta with CAP-11; the key keeps its name so
   * saved settings still apply.)
   * @since v0.3.0
   */
  importAndRetranscribe: boolean;
}

export const DEFAULT_BETA_FEATURES: BetaFeatures = {
  importAndRetranscribe: true, // Default: enabled
};


/**
 * Human-readable feature names for UI display
 */
export const BETA_FEATURE_NAMES: Record<keyof BetaFeatures, string> = {
  importAndRetranscribe: 'Retranscribe',
};

/**
 * Feature descriptions for UI tooltips/help text
 */
export const BETA_FEATURE_DESCRIPTIONS: Record<keyof BetaFeatures, string> = {
  importAndRetranscribe: 'Retranscribe an existing meeting with a different transcription model.',
};

/**
 * Type-safe feature key union
 * This ensures only valid feature keys can be used
 */
export type BetaFeatureKey = keyof BetaFeatures;

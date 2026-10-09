/** User-visible strings of the app shell (root layout). */
export const shellCopy = {
  finishSetupTitle: 'Please complete setup first',
  finishSetupDescription: 'You need to finish onboarding before you can start recording.',
  importDisabledTitle: 'Beta feature disabled',
  importDisabledDescription:
    'Enable "Import Audio & Retranscribe" in Settings > Beta to use this feature.',
  unsupportedDropTitle: 'Please drop an audio file',
  supportedFormats: (formats: string) => `Supported formats: ${formats}`,
} as const;

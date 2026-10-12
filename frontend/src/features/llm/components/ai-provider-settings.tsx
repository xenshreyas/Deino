/**
 * Settings → Notes → AI (UX-11): who writes the notes, with onboarding's
 * three choices (Claude, OpenAI, On this Mac), a key field and one "Check".
 * Groq, OpenRouter, Ollama and custom endpoints sit under "Other
 * providers"; the per-task models are Settings → Advanced. Notes are always
 * written after a meeting, so there is no switch for it.
 */
import { OtherProviders } from '@/components/SummaryModelSettings';
import { NotesProvider } from '@/features/onboarding';
import { type LlmProvider, useSettings } from '@/features/settings';
import { MoreOptions } from '@/ui/more-options';
import { SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { aiProviderCopy as copy } from '../copy';
import { useLlmSettingsChanges } from '../queries';

/** The providers the three choices cover; any other is set under "Other providers". */
const PICKER_PROVIDERS: readonly LlmProvider[] = ['claude', 'openai', 'builtin-ai'];

/** Settings → Notes → AI. */
export function AiProviderSettings() {
  useLlmSettingsChanges();
  const settings = useSettings();
  const ai = settings.data?.ai;
  return (
    <SettingsSection title={copy.title} description={copy.description}>
      {ai === undefined ? (
        <SkeletonText lines={3} />
      ) : (
        <div className="flex flex-col gap-4">
          {PICKER_PROVIDERS.includes(ai.provider) ? null : (
            <p className="text-sm text-fg-muted">{copy.otherInUse(copy.names[ai.provider])}</p>
          )}
          <NotesProvider key={ai.provider} ai={ai} heading={false} />
        </div>
      )}
      <MoreOptions label={copy.otherProviders}>
        <p className="text-xs text-fg-muted">{copy.otherDescription}</p>
        <OtherProviders />
      </MoreOptions>
    </SettingsSection>
  );
}

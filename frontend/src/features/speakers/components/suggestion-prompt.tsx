/**
 * "Is this Maya Chen?" under a speaker the `identify_speakers` stage was
 * unsure of: one click names the speaker for all segments (and saves the
 * voice), the other forgets the guess.
 */
import { Button } from '@/ui/button';

import { speakersCopy } from '../copy';
import type { Suggestion } from '../types';

/** Props of {@link SuggestionPrompt}. */
export interface SuggestionPromptProps {
  suggestion: Suggestion;
  onConfirm: () => void;
  onDismiss: () => void;
  /** True while an answer is being saved. */
  busy: boolean;
}

/** The inline question with Yes and No. */
export function SuggestionPrompt({
  suggestion,
  onConfirm,
  onDismiss,
  busy,
}: SuggestionPromptProps) {
  return (
    <div className="mt-2 flex items-center gap-1 rounded-md bg-accent-soft py-1 pr-1 pl-2.5">
      <p
        className="min-w-0 flex-1 truncate text-xs text-fg"
        title={speakersCopy.suggestion(suggestion.person.name)}
      >
        {speakersCopy.suggestion(suggestion.person.name)}
      </p>
      <Button
        size="sm"
        variant="secondary"
        className="h-6 px-2 text-xs"
        disabled={busy}
        onClick={onConfirm}
      >
        {speakersCopy.yes}
      </Button>
      <Button
        size="sm"
        variant="ghost"
        className="h-6 px-2 text-xs"
        disabled={busy}
        onClick={onDismiss}
      >
        {speakersCopy.no}
      </Button>
    </div>
  );
}

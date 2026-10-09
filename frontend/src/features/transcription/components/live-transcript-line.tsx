import { copy } from '../copy';
import { useLiveTranscript } from '../hooks/use-live-transcript';

/**
 * The words being spoken right now, under the transcript. Committed words
 * are final; a tentative tail (other engines) is shown muted. Renders
 * nothing between segments.
 */
export function LiveTranscriptLine() {
  const live = useLiveTranscript();
  if (live === null) return null;
  return (
    <p aria-label={copy.live} aria-live="polite" className="px-4 py-2 text-sm leading-relaxed">
      {live.committed}
      {live.tentative === '' ? null : <span className="text-fg-muted"> {live.tentative}</span>}
    </p>
  );
}

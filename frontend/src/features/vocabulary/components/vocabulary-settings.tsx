/**
 * Settings → Vocabulary (TRN-05): the words and names Deino should spell
 * the user's way, and plain replacements. People and companies join
 * automatically, so the page lists only what the user added.
 */
import { SkeletonText } from '@/ui/skeleton';

import { vocabularyCopy as copy } from '../copy';
import { byKind } from '../entries';
import { useVocabulary } from '../queries';
import { EntryList } from './entry-list';

/** Settings → Vocabulary. */
export function VocabularySettings() {
  const vocabulary = useVocabulary();
  return (
    <div className="flex flex-col gap-8">
      <p className="max-w-prose text-sm text-pretty text-fg-muted">{copy.intro}</p>
      <VocabularyLists vocabulary={vocabulary} />
      <p className="max-w-prose text-xs text-pretty text-fg-subtle">{copy.howItWorks}</p>
    </div>
  );
}

/** Both cards, or their loading or error state. */
function VocabularyLists({ vocabulary }: { vocabulary: ReturnType<typeof useVocabulary> }) {
  if (vocabulary.isPending) return <SkeletonText lines={4} />;
  if (vocabulary.isError) return <p className="text-sm text-danger">{copy.loadError}</p>;
  const { terms, replacements } = byKind(vocabulary.data);
  return (
    <>
      <EntryList kind="term" entries={terms} />
      <EntryList kind="replacement" entries={replacements} />
    </>
  );
}

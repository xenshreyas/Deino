/**
 * Settings → Notes → Spelling (TRN-05): the words and names Deino should
 * spell the user's way, and plain replacements. People and companies join
 * automatically, so the section lists only what the user added.
 */
import { SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { vocabularyCopy as copy } from '../copy';
import { byKind } from '../entries';
import { useVocabulary } from '../queries';
import { EntryList } from './entry-list';

/** Settings → Notes → Spelling. */
export function VocabularySettings() {
  const vocabulary = useVocabulary();
  return (
    <SettingsSection title={copy.title} description={copy.intro}>
      <div className="flex flex-col gap-6 pt-2">
        <VocabularyLists vocabulary={vocabulary} />
        <p className="max-w-prose text-xs text-pretty text-fg-subtle">{copy.howItWorks}</p>
      </div>
    </SettingsSection>
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

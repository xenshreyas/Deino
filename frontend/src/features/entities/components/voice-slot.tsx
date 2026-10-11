/**
 * A person's voice (CAP-04): whether Deino recognises it, from how many
 * samples, and the way to Settings → People where voice profiles are
 * managed. CAP-04's `VoiceProfiles` lists everyone; this is one person's
 * line of it.
 */
import { Link } from '@tanstack/react-router';

import { useVoiceProfiles } from '@/features/speakers';
import { AudioLines, ICON_PROPS } from '@/ui/icons';

import { entitiesCopy } from '../copy';
import type { VoiceSlotProps } from '../slots';
import { Section } from './entity-sections';

/** The voice line of a person page. */
export function PersonVoice({ personId }: VoiceSlotProps) {
  const profiles = useVoiceProfiles();
  const profile = profiles.data?.find((candidate) => candidate.personId === personId);
  if (profiles.isPending) return null;
  const samples = profile?.voiceprints.length ?? 0;
  return (
    <Section
      id="entity-voice"
      title={entitiesCopy.voice}
      trailing={
        <Link
          to="/settings"
          search={{ section: 'people' }}
          className="text-xs text-fg-muted underline-offset-4 hover:text-fg hover:underline"
        >
          {entitiesCopy.voiceManage}
        </Link>
      }
    >
      <p className="flex items-center gap-2.5 px-3 py-2 text-sm text-fg-muted">
        <AudioLines {...ICON_PROPS} className={samples > 0 ? 'text-accent' : 'text-fg-subtle'} />
        {samples > 0 ? entitiesCopy.voiceKnown(samples) : entitiesCopy.voiceUnknown}
      </p>
    </Section>
  );
}

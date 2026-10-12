/**
 * Settings → Privacy & security → Retention (PRV-01): how long recordings,
 * transcripts and whole meetings are kept. Transcripts cannot be set
 * shorter than recordings (Rust rejects it too): those choices are
 * disabled, the row says why, and a longer recording period raises the
 * transcript's with it.
 */
import { toast } from 'sonner';

import { usePatchSettings, useSettings } from '@/features/settings';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/ui/select';
import { SettingsCard, SettingsRow, SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { retentionCopy } from '../copy';
import { audioChange, keepsAtLeast, PERIODS } from '../periods';
import type { RetentionPeriod, RetentionSettings as Retention } from '../types';

const copy = retentionCopy.settings;

/** The shortest period: every choice is allowed. */
const SHORTEST: RetentionPeriod = 'days7';

/** Props of {@link PeriodSelect}. */
export interface PeriodSelectProps {
  id: string;
  label: string;
  value: RetentionPeriod;
  /** Periods shorter than this are offered but disabled. */
  atLeast: RetentionPeriod;
  onChange: (period: RetentionPeriod) => void;
}

/** One period picker. */
export function PeriodSelect({ id, label, value, atLeast, onChange }: PeriodSelectProps) {
  return (
    <Select
      value={value}
      onValueChange={(next) => {
        const period = PERIODS.find((candidate) => candidate === next);
        if (period !== undefined) onChange(period);
      }}
    >
      <SelectTrigger id={id} aria-label={label} className="w-32">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {PERIODS.map((period) => (
          <SelectItem key={period} value={period} disabled={!keepsAtLeast(period, atLeast)}>
            {copy.period(period)}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

/** Saves a change to `privacy.retention`; failures become a toast. */
function useSaveRetention() {
  const patch = usePatchSettings();
  return (retention: Partial<Retention>) => {
    patch.mutate(
      { privacy: { retention } },
      { onError: (error) => toast.error(copy.saveFailed, { description: error.message }) },
    );
  };
}

/** Which period a row sets. */
type Kind = keyof Retention;

/** The rows, in order. */
const ROWS = [
  { kind: 'audio', id: 'retain-audio', title: copy.audioTitle, hint: copy.audioDescription },
  {
    kind: 'transcript',
    id: 'retain-transcript',
    title: copy.transcriptTitle,
    hint: copy.transcriptDescription,
  },
  {
    kind: 'everything',
    id: 'retain-everything',
    title: copy.everythingTitle,
    hint: copy.everythingDescription,
  },
] as const satisfies readonly { kind: Kind; id: string; title: string; hint: string }[];

/** The change for setting `kind` to `period`. */
function changeFor(kind: Kind, retention: Retention, period: RetentionPeriod): Partial<Retention> {
  if (kind === 'audio') return audioChange(retention, period);
  return kind === 'transcript' ? { transcript: period } : { everything: period };
}

/** The three rows. */
function RetentionCard({ retention }: { retention: Retention }) {
  const save = useSaveRetention();
  return (
    <SettingsCard label={copy.title}>
      {ROWS.map((row) => (
        <SettingsRow key={row.kind} label={row.title} description={row.hint} htmlFor={row.id}>
          <PeriodSelect
            id={row.id}
            label={row.title}
            value={retention[row.kind]}
            atLeast={row.kind === 'transcript' ? retention.audio : SHORTEST}
            onChange={(period) => {
              const change = changeFor(row.kind, retention, period);
              save(change);
              if (row.kind === 'audio' && change.transcript !== undefined) {
                toast.info(copy.transcriptRaised);
              }
            }}
          />
        </SettingsRow>
      ))}
    </SettingsCard>
  );
}

/** "Retention", once the settings are loaded. */
export function RetentionSettings() {
  const settings = useSettings();
  return (
    <SettingsSection title={copy.title} description={copy.description}>
      {settings.data === undefined ? (
        <SkeletonText lines={3} />
      ) : (
        <RetentionCard retention={settings.data.privacy.retention} />
      )}
    </SettingsSection>
  );
}

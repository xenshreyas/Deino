/**
 * The capture switches over `settings.screen`: on for detected meetings
 * (off by default), snapshots, video and its frame rate. Presentational:
 * the container saves each change as a merge patch.
 */
import { SegmentedControl } from '@/ui/segmented-control';
import { Switch } from '@/ui/controls';
import { SettingsCard, SettingsRow } from '@/ui/settings-section';

import { screenCopy } from '../copy';
import type { ScreenSettings } from '../types';

const copy = screenCopy.capture;

/** The frame rates offered: a shared screen rarely needs more than 1. */
const RATES = [
  { value: '1', label: copy.rates[1] },
  { value: '2', label: copy.rates[2] },
  { value: '5', label: copy.rates[5] },
] as const;

/** Props of {@link CaptureCard}. */
export interface CaptureCardProps {
  screen: ScreenSettings;
  /** A save is in flight. */
  saving: boolean;
  onChange: (next: Partial<ScreenSettings>) => void;
}

/** One labelled switch. */
function SwitchRow(props: {
  id: string;
  label: string;
  description: string;
  checked: boolean;
  disabled: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <SettingsRow label={props.label} description={props.description} htmlFor={props.id}>
      <Switch
        id={props.id}
        checked={props.checked}
        disabled={props.disabled}
        onCheckedChange={props.onChange}
      />
    </SettingsRow>
  );
}

/** The video switch, and its frame rate while it is on. */
function VideoRows({
  screen,
  disabled,
  onChange,
}: {
  screen: ScreenSettings;
  disabled: boolean;
  onChange: (next: Partial<ScreenSettings>) => void;
}) {
  return (
    <>
      <SwitchRow
        id="screen-video"
        label={copy.video}
        description={copy.videoHint}
        checked={screen.video}
        disabled={disabled}
        onChange={(video) => {
          onChange({ video });
        }}
      />
      {screen.detectedMeetings && screen.video ? (
        <SettingsRow label={copy.frameRate}>
          <SegmentedControl
            label={copy.frameRate}
            segments={RATES}
            value={String(screen.videoFps)}
            onValueChange={(fps) => {
              onChange({ videoFps: Number(fps) });
            }}
          />
        </SettingsRow>
      ) : null}
    </>
  );
}

/** "Keep snapshots", and describing them with Claude (off by default). */
function SnapshotRows({ screen, saving, onChange }: CaptureCardProps) {
  return (
    <>
      <SwitchRow
        id="screen-snapshots"
        label={copy.snapshots}
        description={copy.snapshotsHint}
        checked={screen.snapshots}
        disabled={saving || !screen.detectedMeetings}
        onChange={(snapshots) => {
          onChange({ snapshots });
        }}
      />
      <SwitchRow
        id="screen-describe"
        label={copy.describe}
        description={copy.describeHint}
        checked={screen.describeSnapshots}
        disabled={saving || !screen.snapshots}
        onChange={(describeSnapshots) => {
          onChange({ describeSnapshots });
        }}
      />
    </>
  );
}

/** The capture settings. */
export function CaptureCard({ screen, saving, onChange }: CaptureCardProps) {
  const on = screen.detectedMeetings;
  return (
    <>
      <SettingsCard label={copy.title}>
        <SwitchRow
          id="screen-detected"
          label={copy.detected}
          description={on ? copy.detectedOn : copy.detectedOff}
          checked={on}
          disabled={saving}
          onChange={(detectedMeetings) => {
            onChange({ detectedMeetings });
          }}
        />
        <SnapshotRows screen={screen} saving={saving} onChange={onChange} />
        <VideoRows screen={screen} disabled={saving || !on} onChange={onChange} />
      </SettingsCard>
      {on ? <p className="px-1 text-xs text-fg-muted">{copy.consent}</p> : null}
    </>
  );
}

/**
 * Settings → Recording, "While recording": the record panel (CAP-10) and
 * when a recording ends on its own (CAP-07). Values live in the typed
 * settings (`recording.showPanel`, `recording.autoStop`).
 */
import { Label, Switch } from '@/ui/controls';
import { SkeletonText } from '@/ui/skeleton';

import { recordingSettingsCopy as copy } from '../copy';
import { usePatchSettings, useSettings } from '../queries';
import type { Settings } from '../types';

/** One switch with its label and description. */
function SwitchRow({
  id,
  label,
  description,
  checked,
  disabled = false,
  onChange,
}: {
  id: string;
  label: string;
  description: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <div className="grid min-h-[52px] grid-cols-[1fr_auto] items-center gap-4 px-4 py-3">
      <div className="flex flex-col gap-0.5">
        <Label htmlFor={id}>{label}</Label>
        <p className="text-xs text-fg-muted">{description}</p>
      </div>
      <Switch id={id} checked={checked} disabled={disabled} onCheckedChange={onChange} />
    </div>
  );
}

/** The auto-stop switches; the calendar rule needs auto-stop on. */
function AutoStopRows({ recording }: { recording: Settings['recording'] }) {
  const patch = usePatchSettings();
  const { autoStop } = recording;
  const save = (change: Partial<Settings['recording']['autoStop']>) => {
    patch.mutate({ recording: { autoStop: { ...autoStop, ...change } } });
  };
  return (
    <>
      <SwitchRow
        id="recording-auto-stop"
        label={copy.autoStopTitle}
        description={copy.autoStopDescription}
        checked={autoStop.enabled}
        onChange={(enabled) => {
          save({ enabled });
        }}
      />
      <SwitchRow
        id="recording-auto-stop-calendar"
        label={copy.calendarTitle}
        description={copy.calendarDescription}
        checked={autoStop.afterCalendarEnd}
        disabled={!autoStop.enabled}
        onChange={(afterCalendarEnd) => {
          save({ afterCalendarEnd });
        }}
      />
      <SwitchRow
        id="recording-next-meeting"
        label={copy.nextMeetingTitle}
        description={copy.nextMeetingDescription}
        checked={autoStop.offerNextMeeting}
        onChange={(offerNextMeeting) => {
          save({ offerNextMeeting });
        }}
      />
    </>
  );
}

/** The record panel and auto-stop, in one card. */
export function RecordingBehaviour() {
  const { data: settings } = useSettings();
  const patch = usePatchSettings();
  if (settings === undefined) return <SkeletonText lines={4} />;
  return (
    <section className="flex flex-col gap-2">
      <h2 className="text-sm font-semibold text-fg">{copy.heading}</h2>
      <div className="divide-y divide-border rounded-lg border border-border bg-surface">
        <SwitchRow
          id="recording-panel"
          label={copy.panelTitle}
          description={copy.panelDescription}
          checked={settings.recording.showPanel}
          onChange={(showPanel) => {
            patch.mutate({ recording: { showPanel } });
          }}
        />
        <AutoStopRows recording={settings.recording} />
      </div>
    </section>
  );
}

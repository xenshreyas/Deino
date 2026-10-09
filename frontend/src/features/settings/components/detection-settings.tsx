/**
 * Settings → Meeting detection (CAP-06): detection on or off, the
 * auto-start countdown, and the apps that never prompt. Values live in the
 * typed settings (`detection`); the detection feature reports what this Mac
 * supports (Accessibility for browser URLs).
 */
import type { ReactNode } from 'react';

import { detectionCopy, type IgnoredApp, useDetectionOverview } from '@/features/detection';
import { Label, Switch } from '@/ui/controls';
import { IconButton } from '@/ui/icon-button';
import { ICON_PROPS, Info, X } from '@/ui/icons';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/ui/select';
import { SkeletonText } from '@/ui/skeleton';

import { usePatchSettings, useSettings } from '../queries';
import type { Settings } from '../types';

const copy = detectionCopy.settings;

/** The countdowns offered, in seconds (the settings allow 3 to 30). */
const COUNTDOWNS = [5, 10, 15, 30] as const;

/** One setting: label and description on the left, its control on the right. */
function SettingRow({
  id,
  label,
  description,
  control,
}: {
  id: string;
  label: string;
  description: string;
  control: ReactNode;
}) {
  return (
    <div className="grid min-h-[52px] grid-cols-[1fr_auto] items-center gap-4 px-4 py-3">
      <div className="flex flex-col gap-0.5">
        <Label htmlFor={id}>{label}</Label>
        <p className="text-xs text-fg-muted">{description}</p>
      </div>
      {control}
    </div>
  );
}

/** The auto-start countdown picker. */
function CountdownPicker({
  seconds,
  disabled,
  onChange,
}: {
  seconds: number;
  disabled: boolean;
  onChange: (seconds: number) => void;
}) {
  return (
    <Select
      value={String(seconds)}
      disabled={disabled}
      onValueChange={(value) => {
        onChange(Number(value));
      }}
    >
      <SelectTrigger aria-label={copy.countdownLabel} className="w-32">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {COUNTDOWNS.map((option) => (
          <SelectItem key={option} value={String(option)}>
            {`${option} ${copy.countdownUnit}`}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

/** Saves a change to the detection section. */
type SaveDetection = (change: Partial<Settings['detection']>) => void;

/** Auto-start: its countdown and its switch, usable only while detection is on. */
function AutoStartControls({
  detection,
  save,
}: {
  detection: Settings['detection'];
  save: SaveDetection;
}) {
  const { enabled, autoStart } = detection;
  return (
    <div className="flex items-center gap-3">
      <CountdownPicker
        seconds={autoStart.countdownSeconds}
        disabled={!enabled || !autoStart.enabled}
        onChange={(seconds) => {
          save({ autoStart: { ...autoStart, countdownSeconds: seconds } });
        }}
      />
      <Switch
        id="detection-auto-start"
        checked={autoStart.enabled}
        disabled={!enabled}
        onCheckedChange={(checked) => {
          save({ autoStart: { ...autoStart, enabled: checked } });
        }}
      />
    </div>
  );
}

/** Detection and auto-start, in one card. */
function BehaviourCard({ detection }: { detection: Settings['detection'] }) {
  const patch = usePatchSettings();
  const save: SaveDetection = (change) => {
    patch.mutate({ detection: change });
  };
  return (
    <div className="divide-y divide-border rounded-lg border border-border bg-surface">
      <SettingRow
        id="detection-enabled"
        label={copy.detectTitle}
        description={copy.detectDescription}
        control={
          <Switch
            id="detection-enabled"
            checked={detection.enabled}
            onCheckedChange={(checked) => {
              save({ enabled: checked });
            }}
          />
        }
      />
      <SettingRow
        id="detection-auto-start"
        label={copy.autoStartTitle}
        description={copy.autoStartDescription(detection.autoStart.countdownSeconds)}
        control={<AutoStartControls detection={detection} save={save} />}
      />
    </div>
  );
}

/** The ignore list, or its resting state. */
function IgnoredApps({ apps }: { apps: readonly IgnoredApp[] }) {
  const patch = usePatchSettings();
  const stopIgnoring = (bundleId: IgnoredApp['bundleId']) => {
    patch.mutate({ detection: { ignoredApps: apps.filter((app) => app.bundleId !== bundleId) } });
  };
  if (apps.length === 0) {
    return (
      <div className="rounded-lg border border-border px-4 py-5 text-center">
        <p className="text-sm font-semibold text-fg">{copy.ignoredEmptyTitle}</p>
        <p className="mt-1 text-xs text-fg-muted">{copy.ignoredEmptyBody}</p>
      </div>
    );
  }
  return (
    <ul className="divide-y divide-border rounded-lg border border-border bg-surface">
      {apps.map((app) => (
        <li key={app.bundleId} className="flex h-14 items-center gap-3 pr-2 pl-4">
          <div className="flex min-w-0 flex-1 flex-col">
            <span className="truncate text-sm text-fg">{app.name}</span>
            <span className="truncate text-xs text-fg-muted">{app.bundleId}</span>
          </div>
          <IconButton
            icon={X}
            label={copy.stopIgnoring(app.name)}
            onClick={() => {
              stopIgnoring(app.bundleId);
            }}
          />
        </li>
      ))}
    </ul>
  );
}

/** A quiet note under the switches: what this Mac can or cannot do. */
function SupportNote() {
  const { data: overview } = useDetectionOverview();
  if (overview === undefined || (overview.supported && overview.readsBrowserUrls)) return null;
  return (
    <p className="flex items-start gap-2 text-xs text-fg-muted">
      <Info {...ICON_PROPS} className="mt-px shrink-0" />
      {overview.supported ? copy.browserHint : copy.unsupported}
    </p>
  );
}

/** Settings → Meeting detection. */
export function DetectionSettingsPage() {
  const { data: settings } = useSettings();
  if (settings === undefined) return <SkeletonText lines={4} />;
  return (
    <div className="flex flex-col gap-8">
      <section className="flex flex-col gap-2">
        <BehaviourCard detection={settings.detection} />
        <SupportNote />
      </section>
      <section aria-labelledby="detection-ignored" className="flex flex-col gap-2">
        <h3 id="detection-ignored" className="text-sm font-semibold text-fg">
          {copy.ignoredTitle}
        </h3>
        <p className="text-xs text-fg-muted">{copy.ignoredDescription}</p>
        <IgnoredApps apps={settings.detection.ignoredApps} />
      </section>
    </div>
  );
}

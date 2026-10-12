/**
 * Settings → Recording → Meeting detection (CAP-06): detection on or off,
 * then under "More options" (UX-15) the auto-start countdown and the apps
 * that never prompt. Values live in the typed settings (`detection`); the
 * detection feature reports what this Mac supports (Accessibility for
 * browser URLs, linked to the Permissions card, UX-10).
 */
import type { ReactNode } from 'react';

import { detectionCopy, type IgnoredApp, useDetectionOverview } from '@/features/detection';
import { Label, Switch } from '@/ui/controls';
import { IconButton } from '@/ui/icon-button';
import { ICON_PROPS, Info, X } from '@/ui/icons';
import { MoreOptions } from '@/ui/more-options';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/ui/select';
import { SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { permissionLinkCopy } from '../copy';

import { usePatchSettings, useSettings } from '../queries';
import type { Settings } from '../types';
import { PermissionLink } from './permission-link';

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

/** Saves a change to the detection section as a merge patch. */
function useSaveDetection(): SaveDetection {
  const patch = usePatchSettings();
  return (change) => {
    patch.mutate({ detection: change });
  };
}

/** The one everyday switch: detection on or off. */
function DetectCard({ detection }: { detection: Settings['detection'] }) {
  const save = useSaveDetection();
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
    </div>
  );
}

/** Auto-start with its countdown, under "More options". */
function AutoStartCard({ detection }: { detection: Settings['detection'] }) {
  const save = useSaveDetection();
  return (
    <div className="divide-y divide-border rounded-lg border border-border bg-surface">
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

/**
 * A quiet line under the switch: "Needs Accessibility" while browser calls
 * cannot be recognised quickly, or that this Mac cannot detect meetings.
 */
function SupportNote({ enabled }: { enabled: boolean }) {
  const { data: overview } = useDetectionOverview();
  if (overview === undefined || !enabled) return null;
  if (!overview.supported) {
    return (
      <p className="flex items-start gap-2 px-1 text-xs text-fg-muted">
        <Info {...ICON_PROPS} className="mt-px shrink-0" />
        {copy.unsupported}
      </p>
    );
  }
  return overview.readsBrowserUrls ? null : (
    <PermissionLink permission={permissionLinkCopy.accessibility} />
  );
}

/** Settings → Recording → Meeting detection. */
export function DetectionSettingsPage() {
  const { data: settings } = useSettings();
  return (
    <SettingsSection title={copy.sectionTitle} description={copy.sectionDescription}>
      {settings === undefined ? (
        <SkeletonText lines={2} />
      ) : (
        <>
          <DetectCard detection={settings.detection} />
          <SupportNote enabled={settings.detection.enabled} />
          <MoreOptions>
            <AutoStartCard detection={settings.detection} />
            <section aria-labelledby="detection-ignored" className="flex flex-col gap-2">
              <h4 id="detection-ignored" className="text-sm font-semibold text-fg">
                {copy.ignoredTitle}
              </h4>
              <p className="text-xs text-fg-muted">{copy.ignoredDescription}</p>
              <IgnoredApps apps={settings.detection.ignoredApps} />
            </section>
          </MoreOptions>
        </>
      )}
    </SettingsSection>
  );
}

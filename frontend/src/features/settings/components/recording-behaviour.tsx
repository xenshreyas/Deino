/**
 * Settings → Recording: the microphone Deino records and the record panel
 * (CAP-10), then when a recording ends on its own (CAP-07), its rarer rules
 * under "More options". Values live in the typed settings
 * (`audio.preferredMicDevice`, `recording.showPanel`, `recording.autoStop`).
 * The inherited switches that duplicated or defeated defaults are gone
 * (UX-13); their fields stay in the settings document.
 */
import { MoreOptions } from '@/ui/more-options';
import { SettingsCard, SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { recordingSettingsCopy as copy } from '../copy';
import { useSettings } from '../queries';
import type { Settings } from '../types';
import { MicrophoneRow } from './microphone-row';
import { useSaveSettings } from './save-settings';
import { SwitchRow } from './switch-row';

type AutoStop = Settings['recording']['autoStop'];

/** Saves part of `recording.autoStop`. */
function useSaveAutoStop(autoStop: AutoStop) {
  const { save } = useSaveSettings();
  return (change: Partial<AutoStop>) => {
    save({ recording: { autoStop: { ...autoStop, ...change } } });
  };
}

/** The calendar rule (needs auto-stop on) and the next-meeting offer. */
function MoreAutoStopRows({ autoStop }: { autoStop: AutoStop }) {
  const save = useSaveAutoStop(autoStop);
  return (
    <SettingsCard label={copy.autoStopMore}>
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
    </SettingsCard>
  );
}

/** The auto-stop switch, then its rarer rules under "More options". */
function AutoStopBody({ autoStop }: { autoStop: AutoStop }) {
  const save = useSaveAutoStop(autoStop);
  return (
    <>
      <SettingsCard label={copy.autoStopSection}>
        <SwitchRow
          id="recording-auto-stop"
          label={copy.autoStopTitle}
          description={copy.autoStopDescription}
          checked={autoStop.enabled}
          onChange={(enabled) => {
            save({ enabled });
          }}
        />
      </SettingsCard>
      <MoreOptions>
        <MoreAutoStopRows autoStop={autoStop} />
      </MoreOptions>
    </>
  );
}

/** "End automatically", once the settings are loaded. */
export function AutoStopSettings() {
  const { data: settings } = useSettings();
  return (
    <SettingsSection title={copy.autoStopSection} description={copy.autoStopSectionDescription}>
      {settings === undefined ? (
        <SkeletonText lines={2} />
      ) : (
        <AutoStopBody autoStop={settings.recording.autoStop} />
      )}
    </SettingsSection>
  );
}

/** The microphone and the record panel. */
export function RecordingBehaviour() {
  const { data: settings } = useSettings();
  const { save } = useSaveSettings();
  return (
    <SettingsSection title={copy.heading} description={copy.headingDescription}>
      {settings === undefined ? (
        <SkeletonText lines={2} />
      ) : (
        <SettingsCard label={copy.heading}>
          <MicrophoneRow preferred={settings.audio.preferredMicDevice} />
          <SwitchRow
            id="recording-panel"
            label={copy.panelTitle}
            description={copy.panelDescription}
            checked={settings.recording.showPanel}
            onChange={(showPanel) => {
              save({ recording: { showPanel } });
            }}
          />
        </SettingsCard>
      )}
    </SettingsSection>
  );
}

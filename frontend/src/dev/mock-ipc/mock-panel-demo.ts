/**
 * Demo scenes for the record panel in `pnpm dev:mock`
 * (`record-panel.html?demo=<scene>&theme=<light|dark>`): a recording is
 * started on the mock recorder, the meter is fed a speech-like wobble, and
 * the scene's state is applied, so every look of the pill can be seen and
 * screenshotted in a plain browser.
 */
import type * as Generated from '@/lib/ipc/generated/recording';

import { setAutoStop } from './mock-record-panel';
import { commands, events } from './recording';

/** The scenes the page can show. */
export const PANEL_SCENES = [
  'collapsed',
  'expanded',
  'warning',
  'paused',
  'ending',
  'split',
] as const;
export type PanelScene = (typeof PANEL_SCENES)[number];

/** Whether `value` names a scene. */
export function isPanelScene(value: string | null): value is PanelScene {
  return PANEL_SCENES.some((scene) => scene === value);
}

const NO_MIC_AUDIO: Generated.HealthIssue = { kind: 'noAudio', channel: 'mic' };

/** A speech-like level: two slow waves and a little jitter. */
function wobble(step: number, phase: number): number {
  const slow = Math.abs(Math.sin(step / 7 + phase)) * 0.18;
  const syllable = Math.abs(Math.sin(step / 2.3 + phase * 3)) * 0.12;
  return slow + syllable;
}

/** Feeds the meter about 30 times a second. */
function feedLevels(): void {
  let step = 0;
  window.setInterval(() => {
    step += 1;
    const levels = { microphone: wobble(step, 0), system: wobble(step, 1.7) * 0.7 };
    void events.audioLevel.emit({ levels });
  }, 33);
}

/** Starts a recording and shows `scene`. */
export async function startPanelDemo(scene: PanelScene): Promise<void> {
  await commands.start('Design review');
  feedLevels();
  const state = await commands.state();
  if (state.status !== 'ok') return;
  const inFuture = (seconds: number) => new Date(Date.now() + seconds * 1000).toISOString();
  const show: Record<PanelScene, () => Promise<unknown>> = {
    collapsed: () => Promise.resolve(),
    expanded: () => events.panelPointer.emit({ inside: true }),
    warning: () =>
      events.recordingState.emit({ snapshot: { ...state.data, issues: [NO_MIC_AUDIO] } }),
    paused: () => commands.pause(),
    ending: () =>
      Promise.resolve(
        setAutoStop({ ending: { reason: 'micReleased', stopAt: inFuture(12) }, split: null }),
      ),
    split: () => {
      const split: Generated.SplitOffer = {
        eventId: 'event-demo' as Generated.CalendarEventId,
        title: 'Weekly sales sync',
        startsAt: new Date().toISOString(),
      };
      return Promise.resolve(setAutoStop({ ending: null, split }));
    },
  };
  await show[scene]();
}

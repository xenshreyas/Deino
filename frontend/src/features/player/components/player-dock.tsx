/**
 * The docked player (Circleback §7.1/§7.6, ui.md §4.2 "Bottom: audio
 * player"): one calm 52px row under the meeting column. Back 15 s, a round
 * play button in ink, forward 15 s, the time, the scrubber with who spoke
 * when, the length, then the quiet text menus (speed, track), the video
 * toggle when there is a screen recording, and Download. Presentational:
 * state in, intents out; `MeetingPlayer` feeds it.
 */
import { formatElapsed } from '@/features/recording';
import { cn } from '@/ui/cn';
import { ICON_PROPS, Pause, Play, RotateCcw, RotateCw, Video } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';
import { Hint } from '@/ui/tooltip';

import { playerCopy } from '../copy';
import type { AudioSource, MediaSource, MeetingId, PlayerStatus } from '../types';
import { DownloadMenu, SourceMenu, SpeedMenu } from './dock-menus';
import { Scrubber } from './scrubber';

/** How far the skip buttons and ←/→ move, in milliseconds. */
export const SKIP_MS = 15_000;

/** What the dock shows. */
export interface PlayerDockState {
  status: PlayerStatus;
  playing: boolean;
  time: number;
  duration: number;
  rate: number;
  source: AudioSource;
  sources: readonly AudioSource[];
  /** Every file, for Download. */
  files: readonly MediaSource[];
  /** The speaker bands' gradient, if the transcript has lines. */
  bands: string | null;
  /** Whether there is a screen recording, and whether it shows. */
  video: { shown: boolean } | null;
}

/** What the dock asks for. */
export interface PlayerDockIntents {
  onToggle: () => void;
  onSkip: (deltaMs: number) => void;
  onSeek: (ms: number) => void;
  onRate: (rate: number) => void;
  onSource: (source: AudioSource) => void;
  onToggleVideo: () => void;
}

/** Props of {@link PlayerDock}. */
export type PlayerDockProps = { meetingId: MeetingId; state: PlayerDockState } & PlayerDockIntents;

/** The round play / pause button. */
function PlayButton({ playing, onToggle }: { playing: boolean; onToggle: () => void }) {
  const label = playing ? playerCopy.pause : playerCopy.play;
  const Glyph = playing ? Pause : Play;
  return (
    <Hint label={label} shortcut={playerCopy.playShortcut} side="top">
      <button
        type="button"
        aria-label={label}
        onClick={onToggle}
        className="flex size-8 shrink-0 items-center justify-center rounded-full bg-fg text-bg transition-[opacity,transform] duration-(--dur-fast) ease-out hover:opacity-85 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus-ring active:scale-95"
      >
        <Glyph {...ICON_PROPS} size={14} fill="currentColor" className={playing ? '' : 'ml-px'} />
      </button>
    </Hint>
  );
}

/** Back 15 s, play / pause, forward 15 s. */
function Transport({ state, onToggle, onSkip }: PlayerDockProps) {
  return (
    <div className="flex shrink-0 items-center gap-1">
      <IconButton
        icon={RotateCcw}
        label={playerCopy.back}
        shortcut={playerCopy.backShortcut}
        tooltipSide="top"
        onClick={() => {
          onSkip(-SKIP_MS);
        }}
      />
      <PlayButton playing={state.playing} onToggle={onToggle} />
      <IconButton
        icon={RotateCw}
        label={playerCopy.forward}
        shortcut={playerCopy.forwardShortcut}
        tooltipSide="top"
        onClick={() => {
          onSkip(SKIP_MS);
        }}
      />
    </div>
  );
}

/** A clock reading: 12px, tabular, muted. */
function Clock({ ms, className }: { ms: number; className?: string }) {
  return (
    <span className={cn('shrink-0 text-xs text-fg-muted tabular-nums', className)}>
      {formatElapsed(ms / 1000)}
    </span>
  );
}

/** The time, the scrubber and the length; or why it cannot play. */
function Timeline({ state, onSeek }: PlayerDockProps) {
  if (state.status === 'error') {
    return <p className="min-w-0 flex-1 text-xs text-fg-muted">{playerCopy.cannotPlay}</p>;
  }
  return (
    <>
      <Clock ms={state.time} className="w-12 text-right" />
      <Scrubber time={state.time} duration={state.duration} bands={state.bands} onSeek={onSeek} />
      <Clock ms={state.duration} className="w-12" />
    </>
  );
}

/** The dock. */
export function PlayerDock(props: PlayerDockProps) {
  const { state } = props;
  const video = state.video;
  return (
    <div
      role="group"
      aria-label={playerCopy.label}
      className="flex h-13 shrink-0 items-center gap-3 border-t border-border bg-bg px-4"
    >
      <Transport {...props} />
      <Timeline {...props} />
      <div className="flex shrink-0 items-center gap-0.5">
        <SpeedMenu rate={state.rate} onRate={props.onRate} />
        <SourceMenu sources={state.sources} source={state.source} onSource={props.onSource} />
        {video === null ? null : (
          <IconButton
            icon={Video}
            label={video.shown ? playerCopy.hideVideo : playerCopy.showVideo}
            aria-pressed={video.shown}
            className={video.shown ? 'text-fg' : undefined}
            onClick={props.onToggleVideo}
          />
        )}
        <DownloadMenu meetingId={props.meetingId} sources={state.files} />
      </div>
    </div>
  );
}

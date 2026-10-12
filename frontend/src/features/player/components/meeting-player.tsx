/**
 * The meeting page's player (TRN-03), the container behind the dock: it
 * reads the meeting's playable files, owns the hidden `<audio>` element and
 * the screen recording's `<video>`, makes them the page's player (the
 * transcript and the citations drive it through `playerActions`), seeks to a
 * linked moment (`?t=`), and listens for Space and ←/→. A meeting without a
 * recording on this Mac shows no dock at all.
 */
import { useCallback, useMemo, useState } from 'react';

import { useMountEffect } from '@/hooks/useMountEffect';

import { mediaUrl } from '../api';
import { usePlayerKeys } from '../hooks/use-player-keys';
import { bandGradient } from '../model/bands';
import { PlaybackController } from '../model/playback-controller';
import { useMedia } from '../queries';
import { loadPlayer, storeSink, unloadPlayer, usePlaybackStore } from '../store';
import type { AudioSource, MediaFile, MeetingId, SpeakerBand } from '../types';
import { PlayerDock, type PlayerDockIntents, type PlayerDockState, SKIP_MS } from './player-dock';
import { VideoPanel } from './video-panel';

/** Props of {@link MeetingPlayer}. */
export interface MeetingPlayerProps {
  meetingId: MeetingId;
  /** Who spoke when, for the scrubber; none paints a neutral track. */
  bands?: readonly SpeakerBand[] | undefined;
  /** A moment to seek to (`?t=`, in milliseconds); a new value seeks again. */
  seekTo?: number | undefined;
}

/** The audio sources, in the order the track menu lists them. */
const AUDIO: readonly AudioSource[] = ['mixed', 'microphone', 'system'];

/** The audio sources among `files`, in menu order. */
function audioSources(files: readonly MediaFile[]): AudioSource[] {
  return AUDIO.filter((source) => files.some((file) => file.source === source));
}

/** The dock, once the meeting's files are known; nothing without audio. */
export function MeetingPlayer(props: MeetingPlayerProps) {
  const media = useMedia(props.meetingId);
  const files = media.data?.files ?? [];
  if (audioSources(files).length === 0) return null;
  const key = `${props.meetingId}:${files.map((file) => file.path).join('|')}`;
  return <LoadedPlayer key={key} {...props} files={files} />;
}

/**
 * Seeks to a linked moment once, as soon as it can (the controller keeps a
 * seek asked for before the file loads). Remount with `key` per moment.
 */
function SeekToMoment({ controller, at }: { controller: PlaybackController; at: number }) {
  useMountEffect(() => {
    controller.seek(at);
  });
  return null;
}

/** The URL of `source` among `files` (the first file as a fallback). */
function urlOf(files: readonly MediaFile[], source: AudioSource): string {
  const file = files.find((one) => one.source === source) ?? files[0];
  return file === undefined ? '' : mediaUrl(file.path);
}

/** The controller, made the page's player for this mount, and stable element refs. */
function useController(meetingId: MeetingId, files: readonly MediaFile[]) {
  const [first] = useState(() => audioSources(files)[0] ?? 'mixed');
  const [controller] = useState(() => new PlaybackController(storeSink(meetingId)));
  useMountEffect(() => {
    loadPlayer(meetingId, controller, first);
    return () => {
      controller.detach();
      unloadPlayer(meetingId);
    };
  });
  usePlayerKeys(
    {
      toggle: () => {
        controller.toggle();
      },
      skip: (ms) => {
        controller.skip(ms);
      },
    },
    SKIP_MS,
  );
  // Stable refs: React calls a new ref function on every render.
  const attachAudio = useCallback(
    (element: HTMLAudioElement | null) => {
      if (element === null) controller.detach();
      else controller.attach(element, urlOf(files, first));
    },
    [controller, files, first],
  );
  const followVideo = useCallback(
    (element: HTMLVideoElement | null) => {
      controller.follow(element);
    },
    [controller],
  );
  return { controller, attachAudio, followVideo };
}

/** What the dock shows, from the store and the files. */
function useDockState(
  files: readonly MediaFile[],
  bands: readonly SpeakerBand[] | undefined,
  videoShown: boolean,
): PlayerDockState {
  const playback = usePlaybackStore();
  const sources = audioSources(files);
  const duration = playback.duration;
  const gradient = useMemo(
    () => (bands === undefined || duration <= 0 ? null : bandGradient(bands, duration)),
    [bands, duration],
  );
  const hasVideo = files.some((file) => file.source === 'video');
  return {
    status: playback.status,
    playing: playback.playing,
    time: playback.time,
    duration,
    rate: playback.rate,
    source: playback.source ?? sources[0] ?? 'mixed',
    sources,
    files: files.map((file) => file.source),
    bands: gradient,
    video: hasVideo ? { shown: videoShown } : null,
  };
}

/** What the dock's controls do to `controller`. */
function dockIntents(
  controller: PlaybackController,
  files: readonly MediaFile[],
  toggleVideo: () => void,
): PlayerDockIntents {
  return {
    onToggle: () => {
      controller.toggle();
    },
    onSkip: (ms) => {
      controller.skip(ms);
    },
    onSeek: (ms) => {
      controller.seek(ms);
    },
    onRate: (rate) => {
      controller.setRate(rate);
    },
    onSource: (next) => {
      usePlaybackStore.setState({ source: next });
      controller.load(urlOf(files, next));
    },
    onToggleVideo: toggleVideo,
  };
}

/** The player over known files; remounted when they change. */
function LoadedPlayer(props: MeetingPlayerProps & { files: readonly MediaFile[] }) {
  const { meetingId, files } = props;
  const { controller, attachAudio, followVideo } = useController(meetingId, files);
  const [videoShown, setVideoShown] = useState(true);
  const state = useDockState(files, props.bands, videoShown);
  const video = files.find((file) => file.source === 'video');
  const intents = dockIntents(controller, files, () => {
    setVideoShown((shown) => !shown);
  });
  return (
    <div className="relative shrink-0">
      <audio ref={attachAudio} preload="metadata" className="hidden" />
      {video === undefined || !videoShown ? null : (
        <VideoPanel url={mediaUrl(video.path)} videoRef={followVideo} />
      )}
      {props.seekTo === undefined ? null : (
        <SeekToMoment key={props.seekTo} controller={controller} at={props.seekTo} />
      )}
      <PlayerDock meetingId={meetingId} state={state} {...intents} />
    </div>
  );
}

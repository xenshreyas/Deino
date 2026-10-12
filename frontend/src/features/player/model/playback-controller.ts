/**
 * {@link PlaybackController}: one meeting's playback over a media element,
 * the player's only object with behaviour (§3A allows a class for a player
 * controller).
 *
 * It owns the element's source, so switching between the mixed, Me and Them
 * tracks keeps the position, the speed and whether it was playing. A seek
 * asked for before the file's metadata has loaded (a `?t=` link, "Play from
 * here" on a fresh page) is kept and applied the moment it can be, so a seek
 * always lands where it was asked, to the media element's own precision.
 * While playing it reports the position every animation frame (word
 * highlighting needs better than `timeupdate`'s 4 Hz). A screen recording,
 * when there is one, follows the audio: muted, started, paused, sped up and
 * moved with it, and pulled back whenever it drifts.
 */
import type { PlayerStatus } from '../types';

/** What the controller needs of an `<audio>` or `<video>` element. */
export type PlayableMedia = Pick<
  HTMLMediaElement,
  | 'currentTime'
  | 'duration'
  | 'paused'
  | 'playbackRate'
  | 'defaultPlaybackRate'
  | 'readyState'
  | 'src'
  | 'play'
  | 'pause'
  | 'load'
  | 'addEventListener'
  | 'removeEventListener'
>;

/** Where the controller reports what the dock shows. */
export interface PlaybackSink {
  status: (status: PlayerStatus) => void;
  playing: (playing: boolean) => void;
  /** The position, in milliseconds. */
  time: (ms: number) => void;
  /** The length, in milliseconds. */
  duration: (ms: number) => void;
  rate: (rate: number) => void;
}

/** Runs a callback on the next frame (`requestAnimationFrame` in the app). */
export interface FrameScheduler {
  request: (callback: () => void) => number;
  cancel: (handle: number) => void;
}

/** `HTMLMediaElement.HAVE_METADATA`: the duration is known and seeking works. */
const HAVE_METADATA = 1;
/** How far the video may drift from the audio before it is moved back, in seconds. */
const MAX_DRIFT_S = 0.25;
/** Milliseconds per second. */
const MS = 1000;

/** The browser's frame scheduler. */
export const animationFrames: FrameScheduler = {
  request: (callback) => requestAnimationFrame(callback),
  cancel: (handle) => {
    cancelAnimationFrame(handle);
  },
};

/** The element events the controller listens to. */
const EVENTS = ['loadedmetadata', 'play', 'pause', 'ended', 'seeked', 'ratechange', 'error'];

/** Playback of one meeting. */
export class PlaybackController {
  private media: PlayableMedia | null = null;
  private follower: PlayableMedia | null = null;
  /** A position to seek to once the metadata is in, in milliseconds. */
  private pending: number | null = null;
  /** Play once the metadata is in (a source switch while playing, an early play). */
  private resume = false;
  private frame: number | null = null;
  private rate = 1;

  constructor(
    private readonly sink: PlaybackSink,
    private readonly frames: FrameScheduler = animationFrames,
  ) {}

  /** Starts controlling `media`, loading `url`. */
  attach(media: PlayableMedia, url: string): void {
    this.detach();
    this.media = media;
    for (const name of EVENTS) media.addEventListener(name, this.onEvent);
    this.load(url);
  }

  /** Stops controlling the element (it unmounted). */
  detach(): void {
    this.stopFrames();
    if (this.media === null) return;
    for (const name of EVENTS) this.media.removeEventListener(name, this.onEvent);
    this.media.pause();
    this.media = null;
  }

  /** Switches to another file, keeping the position, speed and play state. */
  load(url: string): void {
    const media = this.media;
    if (media === null) return;
    this.pending ??= this.loaded() ? this.position() : null;
    this.resume = !media.paused;
    media.src = url;
    media.defaultPlaybackRate = this.rate;
    media.load();
    this.sink.status('loading');
  }

  /** Makes `video` follow the audio, or stops following (`null`). */
  follow(video: PlayableMedia | null): void {
    this.follower = video;
    if (video === null) return;
    video.playbackRate = this.rate;
    video.currentTime = this.position() / MS;
    if (this.media?.paused === false) void video.play().catch(() => undefined);
  }

  /** The position, in milliseconds (a pending seek counts). */
  position(): number {
    if (this.pending !== null || this.media === null) return this.pending ?? 0;
    return this.media.currentTime * MS;
  }

  /** Moves to `ms` (clamped to the recording), now or once it can. */
  seek(ms: number): void {
    const target = this.clamp(ms);
    this.sink.time(target);
    if (this.media === null || !this.loaded()) {
      this.pending = target;
      return;
    }
    this.media.currentTime = target / MS;
    if (this.follower !== null) this.follower.currentTime = target / MS;
  }

  /** Plays (once loaded, if it is not yet). */
  play(): void {
    if (this.media === null) return;
    if (!this.loaded()) {
      this.resume = true;
      return;
    }
    this.media.play().catch(() => {
      this.sink.playing(false);
    });
  }

  /** Pauses. */
  pause(): void {
    this.resume = false;
    this.media?.pause();
  }

  /** Plays when paused, pauses when playing. */
  toggle(): void {
    if (this.media === null || this.media.paused) this.play();
    else this.pause();
  }

  /** Moves by `deltaMs` (negative goes back). */
  skip(deltaMs: number): void {
    this.seek(this.position() + deltaMs);
  }

  /** Sets the speed of the audio and the video. */
  setRate(rate: number): void {
    this.rate = rate;
    for (const media of [this.media, this.follower]) {
      if (media === null) continue;
      media.defaultPlaybackRate = rate;
      media.playbackRate = rate;
    }
    this.sink.rate(rate);
  }

  private readonly onEvent = (event: Event): void => {
    const media = this.media;
    if (media === null) return;
    if (event.type === 'loadedmetadata') this.onMetadata(media);
    else if (event.type === 'play') this.onPlay(media);
    else if (event.type === 'pause' || event.type === 'ended') this.onPause();
    else if (event.type === 'error') this.sink.status('error');
    else this.publish(media);
  };

  private onMetadata(media: PlayableMedia): void {
    this.sink.duration(finite(media.duration) * MS);
    media.playbackRate = this.rate;
    const pending = this.pending;
    this.pending = null;
    if (pending !== null) this.seek(pending);
    this.sink.status('ready');
    if (this.resume) this.play();
    this.resume = false;
  }

  private onPlay(media: PlayableMedia): void {
    this.sink.playing(true);
    if (this.follower !== null) {
      this.follower.currentTime = media.currentTime;
      void this.follower.play().catch(() => undefined);
    }
    this.startFrames();
  }

  private onPause(): void {
    this.sink.playing(false);
    this.follower?.pause();
    this.stopFrames();
    if (this.media !== null) this.publish(this.media);
  }

  /** Reports the position and keeps the video in step. */
  private publish(media: PlayableMedia): void {
    this.sink.time(media.currentTime * MS);
    this.sink.rate(media.playbackRate);
    const video = this.follower;
    if (video !== null && Math.abs(video.currentTime - media.currentTime) > MAX_DRIFT_S) {
      video.currentTime = media.currentTime;
    }
  }

  private startFrames(): void {
    this.stopFrames();
    const tick = (): void => {
      if (this.media === null) return;
      this.publish(this.media);
      this.frame = this.frames.request(tick);
    };
    this.frame = this.frames.request(tick);
  }

  private stopFrames(): void {
    if (this.frame !== null) this.frames.cancel(this.frame);
    this.frame = null;
  }

  private loaded(): boolean {
    return this.media !== null && this.media.readyState >= HAVE_METADATA;
  }

  private clamp(ms: number): number {
    const length = this.media === null ? Number.NaN : finite(this.media.duration) * MS;
    const upper = this.loaded() && length > 0 ? length : Number.POSITIVE_INFINITY;
    return Math.min(Math.max(0, ms), upper);
  }
}

/** `value`, or 0 for `NaN` and the infinities (a stream's unknown length). */
function finite(value: number): number {
  return Number.isFinite(value) ? value : 0;
}

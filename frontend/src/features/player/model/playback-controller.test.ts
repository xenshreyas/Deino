import { beforeEach, describe, expect, it } from 'vitest';

import type { PlayerStatus } from '../types';
import {
  type FrameScheduler,
  PlaybackController,
  type PlayableMedia,
  type PlaybackSink,
} from './playback-controller';

/** A media element that loads when told and plays without a clock. */
class FakeMedia implements PlayableMedia {
  currentTime = 0;
  duration = Number.NaN;
  paused = true;
  playbackRate = 1;
  defaultPlaybackRate = 1;
  readyState = 0;
  src = '';
  loads = 0;
  private readonly listeners = new Map<string, Set<(event: Event) => void>>();

  play(): Promise<void> {
    this.paused = false;
    this.emit('play');
    return Promise.resolve();
  }

  pause(): void {
    if (this.paused) return;
    this.paused = true;
    this.emit('pause');
  }

  load(): void {
    this.loads += 1;
    this.readyState = 0;
    this.currentTime = 0;
    this.playbackRate = this.defaultPlaybackRate;
    this.paused = true;
  }

  /** The file's metadata arrives. */
  loadMetadata(seconds: number): void {
    this.duration = seconds;
    this.readyState = 1;
    this.emit('loadedmetadata');
  }

  addEventListener(type: string, listener: (event: Event) => void): void {
    const set = this.listeners.get(type) ?? new Set();
    set.add(listener);
    this.listeners.set(type, set);
  }

  removeEventListener(type: string, listener: (event: Event) => void): void {
    this.listeners.get(type)?.delete(listener);
  }

  emit(type: string): void {
    for (const listener of this.listeners.get(type) ?? []) listener(new Event(type));
  }
}

/** Frames that run only when the test says so. */
class ManualFrames implements FrameScheduler {
  private pending = new Map<number, () => void>();
  private next = 1;
  request = (callback: () => void) => {
    this.pending.set(this.next, callback);
    return this.next++;
  };
  cancel = (handle: number) => {
    this.pending.delete(handle);
  };
  /** Runs the frames queued so far. */
  flush(): void {
    const callbacks = [...this.pending.values()];
    this.pending.clear();
    for (const callback of callbacks) callback();
  }
}

/** Everything the controller reported, latest value per channel. */
interface Reported {
  status: PlayerStatus | null;
  playing: boolean;
  time: number;
  duration: number;
  rate: number;
}

function setup() {
  const reported: Reported = { status: null, playing: false, time: 0, duration: 0, rate: 1 };
  const sink: PlaybackSink = {
    status: (status) => (reported.status = status),
    playing: (playing) => (reported.playing = playing),
    time: (ms) => (reported.time = ms),
    duration: (ms) => (reported.duration = ms),
    rate: (rate) => (reported.rate = rate),
  };
  const frames = new ManualFrames();
  const media = new FakeMedia();
  const controller = new PlaybackController(sink, frames);
  controller.attach(media, 'asset://mixed');
  return { reported, frames, media, controller };
}

let world: ReturnType<typeof setup>;
beforeEach(() => {
  world = setup();
});

describe('PlaybackController', () => {
  it('loads the source, then reports the length once the metadata is in', () => {
    const { media, reported } = world;
    expect(media.src).toBe('asset://mixed');
    expect(reported.status).toBe('loading');

    media.loadMetadata(53.4);

    expect(reported.status).toBe('ready');
    expect(reported.duration).toBeCloseTo(53_400);
  });

  it('keeps a seek asked for before the metadata and lands it exactly', () => {
    const { media, controller, reported } = world;
    controller.seek(12_345);
    expect(media.currentTime).toBe(0);

    media.loadMetadata(60);

    expect(media.currentTime * 1000).toBeCloseTo(12_345, 6);
    expect(reported.time).toBe(12_345);
  });

  it('clamps seeks and skips to the recording', () => {
    const { media, controller } = world;
    media.loadMetadata(30);

    controller.seek(-5_000);
    expect(media.currentTime).toBe(0);
    controller.skip(45_000);
    expect(media.currentTime).toBe(30);
    controller.skip(-15_000);
    expect(media.currentTime).toBe(15);
  });

  it('plays once loaded when asked early, and reports the position every frame', () => {
    const { media, controller, frames, reported } = world;
    controller.play();
    expect(media.paused).toBe(true);

    media.loadMetadata(60);
    expect(media.paused).toBe(false);
    media.currentTime = 2.5;
    frames.flush();

    expect(reported.playing).toBe(true);
    expect(reported.time).toBe(2_500);
  });

  it('switches track keeping the position, the speed and playing', () => {
    const { media, controller } = world;
    media.loadMetadata(60);
    controller.setRate(1.5);
    controller.seek(20_000);
    controller.play();

    controller.load('asset://mic');
    media.loadMetadata(60);

    expect(media.src).toBe('asset://mic');
    expect(media.currentTime).toBe(20);
    expect(media.playbackRate).toBe(1.5);
    expect(media.paused).toBe(false);
  });

  it('keeps a video in step: moved, sped up, and pulled back when it drifts', () => {
    const { media, controller, frames } = world;
    const video = new FakeMedia();
    video.readyState = 1;
    media.loadMetadata(60);
    controller.follow(video);
    controller.setRate(2);
    controller.seek(10_000);
    controller.play();

    expect(video.currentTime).toBe(10);
    expect(video.playbackRate).toBe(2);
    expect(video.paused).toBe(false);
    media.currentTime = 11;
    video.currentTime = 12;
    frames.flush();
    expect(video.currentTime).toBe(11);
    controller.pause();
    expect(video.paused).toBe(true);
  });

  it('reports a file it cannot play', () => {
    world.media.emit('error');

    expect(world.reported.status).toBe('error');
  });
});

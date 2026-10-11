/**
 * The screen recording (CAP-09), floating above the dock's right end: a
 * 16:9 frame on the raised surface with the popover shadow, muted (the audio
 * plays the sound), following the audio's time and speed.
 */
import type { Ref } from 'react';

/** Props of {@link VideoPanel}. */
export interface VideoPanelProps {
  url: string;
  /** Receives the element, so the controller can make it follow the audio. */
  videoRef: Ref<HTMLVideoElement>;
}

/** The panel. */
export function VideoPanel({ url, videoRef }: VideoPanelProps) {
  return (
    <div className="pointer-events-none absolute right-4 bottom-16 z-(--z-sticky) w-80 overflow-hidden rounded-lg bg-surface-raised shadow-popover">
      <video
        ref={videoRef}
        src={url}
        muted
        playsInline
        preload="metadata"
        className="aspect-video w-full bg-bg-muted object-contain"
      />
    </div>
  );
}

import { useState } from 'react';

import { events } from '@/lib/ipc/generated/transcription';
import { useIpcEvent } from '@/lib/ipc/events';

import type { LiveTranscript } from '../types';

/**
 * The segment being spoken now, as Rust's `LiveTranscript` event grows it,
 * or `null` when no segment is open (it then arrives as a final transcript line).
 */
export function useLiveTranscript(): LiveTranscript | null {
  const [live, setLive] = useState<LiveTranscript | null>(null);
  useIpcEvent(events.liveTranscript, (payload) => {
    const empty = payload.committed === '' && payload.tentative === '';
    setLive(empty ? null : payload);
  });
  return live;
}

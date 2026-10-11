/**
 * Mid-recording device controls (CAP-10): switch the microphone, record
 * the call's audio or not.
 * Each answers with the recorder's new state, which goes into the cache.
 */
import { useMutation, useQueryClient } from '@tanstack/react-query';

import { recordingApi } from '../api';
import { recordingCopy } from '../copy';
import { recordingKeys, report } from '../queries';
import type { RecordingSnapshot } from '../types';

/** Switch the microphone and turn the call's audio on or off. */
export function useDeviceControls() {
  const client = useQueryClient();
  const remember = (snapshot: RecordingSnapshot) => {
    client.setQueryData(recordingKeys.state(), snapshot);
  };
  return {
    microphone: useMutation({
      mutationFn: recordingApi.setMicrophone,
      onSuccess: remember,
      onError: report(recordingCopy.switchFailed),
    }),
    systemAudio: useMutation({
      mutationFn: recordingApi.setSystemAudio,
      onSuccess: remember,
      onError: report(recordingCopy.switchFailed),
    }),
  };
}

/**
 * The record screen's live transcript: a copy button, the permission notice
 * while idle, and the streaming transcript in a 720px column.
 */
import { useMemo } from 'react';

import { PermissionWarning } from '@/components/PermissionWarning';
import { VirtualizedTranscriptView } from '@/components/VirtualizedTranscriptView';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import { useTranscripts } from '@/contexts/TranscriptContext';
import { usePermissionCheck } from '@/hooks/usePermissionCheck';
import type { Transcript, TranscriptSegmentData } from '@/types';
import { useIsLinux } from '@/hooks/usePlatform';
import { Button } from '@/ui/button';
import { Copy, ICON_PROPS } from '@/ui/icons';

/** Props of {@link TranscriptPanel}. */
interface TranscriptPanelProps {
  /** The transcript is being finalised after Stop. */
  isProcessingStop: boolean;
  isStopping: boolean;
}

/** A live transcript line as the transcript view's segment. */
function toSegment(transcript: Transcript): TranscriptSegmentData {
  return {
    id: transcript.id,
    timestamp: transcript.audio_start_time ?? 0,
    text: transcript.text,
    ...(transcript.audio_end_time === undefined ? {} : { endTime: transcript.audio_end_time }),
    ...(transcript.confidence === undefined ? {} : { confidence: transcript.confidence }),
  };
}

/** The live transcript; reads the transcript and recording contexts. */
export function TranscriptPanel({ isProcessingStop, isStopping }: TranscriptPanelProps) {
  const { transcripts, transcriptContainerRef, copyTranscript } = useTranscripts();
  const { isRecording, isPaused } = useRecordingState();
  const { checkPermissions, isChecking, hasSystemAudio, hasMicrophone } = usePermissionCheck();
  const isLinux = useIsLinux();

  const segments = useMemo(() => transcripts.map(toSegment), [transcripts]);

  return (
    <div ref={transcriptContainerRef} className="flex w-full flex-col overflow-y-auto">
      <div className="mx-auto flex w-full max-w-[720px] flex-col px-6 pb-28">
        <div className="flex h-10 items-center justify-end">
          {transcripts.length > 0 ? (
            <Button variant="ghost" size="sm" onClick={copyTranscript}>
              <Copy {...ICON_PROPS} />
              Copy
            </Button>
          ) : null}
        </div>
        {!isRecording && !isChecking && !isLinux ? (
          <PermissionWarning
            hasMicrophone={hasMicrophone}
            hasSystemAudio={hasSystemAudio}
            onRecheck={() => void checkPermissions()}
            isRechecking={isChecking}
          />
        ) : null}
        <VirtualizedTranscriptView
          segments={segments}
          isRecording={isRecording}
          isPaused={isPaused}
          isProcessing={isProcessingStop}
          isStopping={isStopping}
          enableStreaming={isRecording}
          showConfidence
        />
      </div>
    </div>
  );
}

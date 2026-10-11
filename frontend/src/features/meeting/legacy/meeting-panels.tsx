import { useState, useRef, type ReactNode } from 'react';
import { MeetingSummary, SummaryProcessResponse } from '@/types';
import { toast } from 'sonner';
import { TranscriptPanel } from '@/components/MeetingDetails/TranscriptPanel';
import { SummaryPanel } from '@/components/MeetingDetails/SummaryPanel';
import { ModelConfig } from '@/components/ModelSettingsModal';
import { LegacyMeetingPanelsProvider } from './panels-context';

// Custom hooks
import { useMeetingData } from '@/hooks/meeting-details/useMeetingData';
import { useSummaryGeneration } from '@/hooks/meeting-details/useSummaryGeneration';
import { useLegacyTemplateChoice } from '@/features/templates';
import { useMeetingOperations } from '@/hooks/meeting-details/useMeetingOperations';
import { useConfig } from '@/contexts/ConfigContext';
import { hasVisibleSummaryContent } from '@/lib/summary-content';
import { useShareActions } from '@/features/export';
import { asMeetingId } from '@/features/meetings';

export default function PageContent({
  meeting,
  summaryData,
  initialSummary,
  onMeetingUpdated,
  onRefetchTranscripts,
  // Pagination props for efficient transcript loading
  segments,
  hasMore,
  isLoadingMore,
  totalCount,
  loadedCount,
  onLoadMore,
  children,
}: {
  /** The meeting page whose Notes and Transcript slots show these panels. */
  children: ReactNode;
  meeting: any;
  summaryData: MeetingSummary | null;
  initialSummary: SummaryProcessResponse | null;
  onMeetingUpdated?: () => Promise<void>;
  onRefetchTranscripts?: () => Promise<void>;
  // Pagination props
  segments?: any[];
  hasMore?: boolean;
  isLoadingMore?: boolean;
  totalCount?: number;
  loadedCount?: number;
  onLoadMore?: () => void;
}) {
  console.log('📄 PAGE CONTENT: Initializing with data:', {
    meetingId: meeting.id,
    summaryDataKeys: summaryData ? Object.keys(summaryData) : null,
    transcriptsCount: meeting.transcripts?.length
  });

  // State
  const isRecording = false;

  // Ref to store the modal open function from SummaryGeneratorButtonGroup
  const openModelSettingsRef = useRef<(() => void) | null>(null);

  // Get model config from ConfigContext
  const { modelConfig, setModelConfig, saveModelConfig, isModelConfigLoading } = useConfig();

  // Custom hooks
  const meetingData = useMeetingData({ meeting, summaryData, onMeetingUpdated });
  const templates = useLegacyTemplateChoice(meeting.id);

  // Callback to register the modal open function
  const handleRegisterModalOpen = (openFn: () => void) => {
    console.log('📝 Registering modal open function in PageContent');
    openModelSettingsRef.current = openFn;
  };

  // Callback to trigger modal open (called from error handler)
  const handleOpenModelSettings = () => {
    console.log('🔔 Opening model settings from PageContent');
    if (openModelSettingsRef.current) {
      openModelSettingsRef.current();
    } else {
      console.warn('⚠️ Modal open function not yet registered');
    }
  };

  // Save the model choice to the typed settings (and any typed key to the Keychain)
  const handleSaveModelConfig = async (config?: ModelConfig) => {
    if (!config) return;
    try {
      await saveModelConfig(config);
      toast.success('Model settings saved successfully');
    } catch (error) {
      console.error('Failed to save model config:', error);
      toast.error('Failed to save model settings');
    }
  };

  const summaryGeneration = useSummaryGeneration({
    initialSummary,
    meeting,
    transcripts: meetingData.transcripts,
    modelConfig: modelConfig,
    isModelConfigLoading,
    selectedTemplate: templates.selectedTemplate,
    onMeetingUpdated,
    updateMeetingTitle: meetingData.updateMeetingTitle,
    setAiSummary: meetingData.setAiSummary,
    onOpenModelSettings: handleOpenModelSettings,
  });

  // EXP-01: copies go through the export plugin (the stored notes and transcript).
  const share = useShareActions(asMeetingId(meeting.id));

  const meetingOperations = useMeetingOperations({
    meeting,
  });


  return (
    <LegacyMeetingPanelsProvider
      value={{
        status: 'ready',
        regenerateNotes: meetingData.aiSummary
          ? () => { void summaryGeneration.handleRegenerateSummary(); }
          : null,
        hasSummary: hasVisibleSummaryContent(meetingData.aiSummary),
        transcript: (
            <TranscriptPanel
              transcripts={meetingData.transcripts}
              onCopyTranscript={() => share.copy('transcript')}
              onOpenMeetingFolder={meetingOperations.handleOpenMeetingFolder}
              isRecording={isRecording}
              disableAutoScroll={true}
              usePagination={true}
              segments={segments}
              hasMore={hasMore}
              isLoadingMore={isLoadingMore}
              totalCount={totalCount}
              loadedCount={loadedCount}
              onLoadMore={onLoadMore}
              meetingId={meeting.id}
              meetingFolderPath={meeting.folder_path}
              onRefetchTranscripts={onRefetchTranscripts}
            />
        ),
        notes: (
            <SummaryPanel
              meeting={meeting}
              meetingTitle={meetingData.meetingTitle}
              summaryRef={meetingData.summaryDocumentRef}
              isSaving={meetingData.isSaving}
              isSummaryDirty={meetingData.isSummaryDirty}
              onSaveAll={meetingData.saveAllChanges}
              onCopySummary={async () => { share.copy('markdown'); }}
              aiSummary={meetingData.aiSummary}
              summaryStatus={summaryGeneration.summaryStatus}
              transcripts={meetingData.transcripts}
              modelConfig={modelConfig}
              setModelConfig={setModelConfig}
              onSaveModelConfig={handleSaveModelConfig}
              onGenerateSummary={summaryGeneration.handleGenerateSummary}
              onStopGeneration={summaryGeneration.handleStopGeneration}
              onSaveSummary={meetingData.handleSaveSummary}
              onSummaryChange={meetingData.handleSummaryChange}
              onDirtyChange={meetingData.setIsSummaryDirty}
              summaryError={summaryGeneration.summaryError}
              onRegenerateSummary={summaryGeneration.handleRegenerateSummary}
              getSummaryStatusMessage={summaryGeneration.getSummaryStatusMessage}
              availableTemplates={templates.availableTemplates}
              selectedTemplate={templates.selectedTemplate}
              onTemplateSelect={templates.handleTemplateSelection}
              isModelConfigLoading={isModelConfigLoading}
              onOpenModelSettings={handleRegisterModalOpen}
            />
        ),
      }}
    >
      {children}
    </LegacyMeetingPanelsProvider>
  );
}

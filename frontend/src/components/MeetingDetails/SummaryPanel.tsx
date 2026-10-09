import { MeetingSummary, Summary, Transcript } from '@/types';
import { BlockNoteSummaryView, BlockNoteSummaryViewRef } from '@/components/AISummary/BlockNoteSummaryView';
import { EmptyStateSummary } from '@/components/EmptyStateSummary';
import { ModelConfig } from '@/components/ModelSettingsModal';
import { SummaryGeneratorButtonGroup } from './SummaryGeneratorButtonGroup';
import { SummaryUpdaterButtonGroup } from './SummaryUpdaterButtonGroup';
import { RefObject } from 'react';
import { hasVisibleSummaryContent } from '@/lib/summary-content';
import { SkeletonText } from '@/ui/skeleton';

interface SummaryPanelProps {
  meeting: {
    id: string;
    title: string;
    created_at: string;
  };
  meetingTitle: string;
  isSummaryDirty: boolean;
  summaryRef: RefObject<BlockNoteSummaryViewRef>;
  isSaving: boolean;
  onSaveAll: () => Promise<void>;
  onCopySummary: () => Promise<void>;
  aiSummary: MeetingSummary | null;
  summaryStatus: 'idle' | 'processing' | 'summarizing' | 'regenerating' | 'completed' | 'error';
  transcripts: Transcript[];
  modelConfig: ModelConfig;
  setModelConfig: (config: ModelConfig | ((prev: ModelConfig) => ModelConfig)) => void;
  onSaveModelConfig: (config?: ModelConfig) => Promise<void>;
  onGenerateSummary: (customPrompt: string) => Promise<void>;
  onStopGeneration: () => void;
  customPrompt: string;
  onSaveSummary: (summary: MeetingSummary) => Promise<void>;
  onSummaryChange: (summary: Summary) => void;
  onDirtyChange: (isDirty: boolean) => void;
  summaryError: string | null;
  onRegenerateSummary: () => Promise<void>;
  getSummaryStatusMessage: (status: 'idle' | 'processing' | 'summarizing' | 'regenerating' | 'completed' | 'error') => string;
  availableTemplates: Array<{ id: string, name: string, description: string }>;
  selectedTemplate: string;
  onTemplateSelect: (templateId: string, templateName: string) => void;
  isModelConfigLoading?: boolean;
  onOpenModelSettings?: (openFn: () => void) => void;
}

export function SummaryPanel({
  meeting,
  meetingTitle,
  isSummaryDirty,
  summaryRef,
  isSaving,
  onSaveAll,
  onCopySummary,
  aiSummary,
  summaryStatus,
  transcripts,
  modelConfig,
  setModelConfig,
  onSaveModelConfig,
  onGenerateSummary,
  onStopGeneration,
  customPrompt,
  onSaveSummary,
  onSummaryChange,
  onDirtyChange,
  summaryError,
  onRegenerateSummary,
  getSummaryStatusMessage,
  availableTemplates,
  selectedTemplate,
  onTemplateSelect,
  isModelConfigLoading = false,
  onOpenModelSettings,
}: SummaryPanelProps) {
  const isSummaryLoading = summaryStatus === 'processing' || summaryStatus === 'summarizing' || summaryStatus === 'regenerating';
  const hasSummary = hasVisibleSummaryContent(aiSummary);

  return (
    <div className="flex-1 min-w-0 flex flex-col overflow-hidden h-full w-full @container">
      {/* Top-level actions — always visible, same pattern as TranscriptPanel */}
      <div className="py-2">
        <div className="flex items-center justify-end w-full min-w-0 gap-1 flex-wrap">
          <div className="shrink-0 min-w-0">
            <SummaryGeneratorButtonGroup
              modelConfig={modelConfig}
              setModelConfig={setModelConfig}
              onSaveModelConfig={onSaveModelConfig}
              onGenerateSummary={onGenerateSummary}
              onStopGeneration={onStopGeneration}
              customPrompt={customPrompt}
              summaryStatus={summaryStatus}
              availableTemplates={availableTemplates}
              selectedTemplate={selectedTemplate}
              onTemplateSelect={onTemplateSelect}
              hasTranscripts={transcripts.length > 0}
              hasSummary={hasSummary}
              isModelConfigLoading={isModelConfigLoading}
              onOpenModelSettings={onOpenModelSettings}
            />
          </div>

          {hasSummary && !isSummaryLoading && (
            <div className="shrink-0">
              <SummaryUpdaterButtonGroup
                isSaving={isSaving}
                isDirty={isSummaryDirty}
                onSave={onSaveAll}
                onCopy={onCopySummary}
              />
            </div>
          )}
        </div>
      </div>

      {isSummaryLoading ? (
        <div className="flex flex-col gap-4 py-6">
          <p className="text-xs text-fg-muted">Writing your notes…</p>
          <SkeletonText lines={5} />
        </div>
      ) : !hasSummary ? (
        <EmptyStateSummary
          onGenerate={() => onGenerateSummary(customPrompt)}
          hasModel={modelConfig.provider !== null && modelConfig.model !== null}
          isGenerating={isSummaryLoading}
          error={summaryError}
        />
      ) : (
        <div className="flex-1 overflow-y-auto overflow-x-auto min-h-0">
          <div className="py-4 w-full">
            <BlockNoteSummaryView
              ref={summaryRef}
              summaryData={aiSummary}
              onSave={onSaveSummary}
              onSummaryChange={onSummaryChange}
              onDirtyChange={onDirtyChange}
              status={summaryStatus}
              error={summaryError}
              onRegenerateSummary={() => {
                onRegenerateSummary();
              }}
              meeting={{
                id: meeting.id,
                title: meetingTitle,
                created_at: meeting.created_at
              }}
            />
          </div>
          {summaryStatus !== 'idle' && summaryStatus !== 'completed' && (
            <p className={`pb-6 text-xs ${summaryStatus === 'error' ? 'text-danger' : 'text-fg-muted'}`}>
              {getSummaryStatusMessage(summaryStatus)}
            </p>
          )}
        </div>
      )}
    </div>
  );
}

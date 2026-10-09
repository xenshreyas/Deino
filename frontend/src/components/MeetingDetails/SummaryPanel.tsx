"use client";

import { MeetingSummary, Summary, Transcript } from '@/types';
import { BlockNoteSummaryView, BlockNoteSummaryViewRef } from '@/components/AISummary/BlockNoteSummaryView';
import { EmptyStateSummary } from '@/components/EmptyStateSummary';
import { ModelConfig } from '@/components/ModelSettingsModal';
import { SummaryGeneratorButtonGroup } from './SummaryGeneratorButtonGroup';
import { SummaryUpdaterButtonGroup } from './SummaryUpdaterButtonGroup';
import { RefObject } from 'react';
import { hasVisibleSummaryContent } from '@/lib/summary-content';

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
    <div className="flex-1 min-w-0 flex flex-col bg-white overflow-hidden h-full w-full @container">
      {/* Top-level actions — always visible, same pattern as TranscriptPanel */}
      <div className="p-4 border-b border-gray-200">
        <div className="flex items-center justify-center w-full min-w-0 gap-2 flex-wrap">
          <div className="flex-shrink-0 min-w-0">
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
            <div className="flex-shrink-0">
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
        <div className="flex items-center justify-center flex-1">
          <div className="text-center">
            <div className="inline-block animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-500 mb-4"></div>
            <p className="text-gray-600">Generating AI Summary...</p>
          </div>
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
          <div className="p-6 w-full">
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
          {summaryStatus !== 'idle' && (
            <div className={`mt-4 p-4 rounded-lg ${summaryStatus === 'error' ? 'bg-red-100 text-red-700' :
              summaryStatus === 'completed' ? 'bg-green-100 text-green-700' :
                'bg-blue-100 text-blue-700'
              }`}>
              <p className="text-sm font-medium">{getSummaryStatusMessage(summaryStatus)}</p>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

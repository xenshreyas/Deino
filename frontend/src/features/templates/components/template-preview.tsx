import { useState } from 'react';

import { type MeetingId, useMeetings } from '@/features/meetings';
import { Button } from '@/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/ui/select';

import { templatesCopy } from '../copy';
import { usePreviewTemplate } from '../queries';
import type { PromptBlock, TemplateContent } from '../types';

/** Props of {@link TemplatePreview}. */
export interface TemplatePreviewProps {
  /** The content as currently edited, saved or not. */
  content: () => TemplateContent;
}

/**
 * Renders the template, as edited, against a past meeting: exactly the
 * prompt parts the AI would receive. No LLM call is made.
 */
export function TemplatePreview({ content }: TemplatePreviewProps) {
  const preview = usePreviewTemplate();
  const [meeting, setMeeting] = useState<MeetingId | null>(null);
  const render = () => {
    if (meeting !== null) preview.mutate({ content: content(), meeting });
  };
  return (
    <section aria-labelledby="template-preview" className="flex flex-col gap-2">
      <div className="flex flex-col gap-0.5">
        <h3 id="template-preview" className="text-sm font-semibold text-fg">
          {templatesCopy.preview}
        </h3>
        <p className="text-xs text-fg-muted">{templatesCopy.previewHint}</p>
      </div>
      <div className="flex items-center gap-2">
        <MeetingSelect value={meeting} onChange={setMeeting} />
        <Button
          variant="secondary"
          disabled={meeting === null || preview.isPending}
          onClick={render}
        >
          {templatesCopy.renderPreview}
        </Button>
      </div>
      {preview.error === null ? null : (
        <p role="alert" className="text-sm text-danger">
          {preview.error.message}
        </p>
      )}
      {preview.data === undefined ? null : <PreviewBlocks blocks={preview.data.blocks} />}
    </section>
  );
}

/** Every meeting, newest first, to preview against. */
function MeetingSelect({
  value,
  onChange,
}: {
  value: MeetingId | null;
  onChange: (meeting: MeetingId | null) => void;
}) {
  const meetings = useMeetings();
  return (
    <Select
      value={value ?? ''}
      onValueChange={(next) => {
        onChange(meetings.data?.find((candidate) => candidate.id === next)?.id ?? null);
      }}
    >
      <SelectTrigger aria-label={templatesCopy.previewMeeting} className="max-w-sm">
        <SelectValue placeholder={templatesCopy.pickMeeting} />
      </SelectTrigger>
      <SelectContent>
        {(meetings.data ?? []).map((candidate) => (
          <SelectItem key={candidate.id} value={candidate.id}>
            {candidate.title}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

/** The prompt, part by part, as the AI reads it. */
function PreviewBlocks({ blocks }: { blocks: PromptBlock[] }) {
  return (
    <ol
      aria-label={templatesCopy.preview}
      className="flex flex-col gap-4 rounded-lg bg-bg-subtle px-4 py-4"
    >
      {blocks.map((block) => (
        <li key={block.heading} className="flex flex-col gap-1">
          <span className="text-xs font-semibold text-fg-muted">{block.heading}</span>
          <p className="text-sm leading-relaxed whitespace-pre-wrap text-fg">{block.body}</p>
        </li>
      ))}
    </ol>
  );
}

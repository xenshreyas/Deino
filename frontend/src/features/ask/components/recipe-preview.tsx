/**
 * Recipes → Preview (AI-04): run the selected recipe on a meeting the user
 * picks and read the answer, with its citations, without saving anything.
 * The `/recipes` route hands this to the Recipes page.
 */
import { useState } from 'react';

import { type Meeting, type MeetingId, useMeetings } from '@/features/meetings';
import type { Recipe } from '@/features/recipes';
import { Button } from '@/ui/button';
import { Label } from '@/ui/controls';
import { ICON_PROPS, Play } from '@/ui/icons';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/ui/select';

import { streamingText } from '../answer-text';
import { askCopy } from '../copy';
import { usePreview } from '../hooks/use-ask';
import { AnswerBody } from './answer-body';

/** How many recent meetings the picker lists. */
const RECENT = 30;

/** Props of {@link RecipePreview}. */
export interface RecipePreviewProps {
  recipe: Recipe;
}

/** The meeting picker, Run and the answer. */
export function RecipePreview({ recipe }: RecipePreviewProps) {
  const meetings = (useMeetings().data ?? []).slice(0, RECENT);
  const [meeting, setMeeting] = useState<MeetingId | undefined>(undefined);
  const preview = usePreview();
  const chosen = meeting ?? meetings[0]?.id;
  return (
    <section className="grid gap-3 border-t border-border pt-6" aria-label={askCopy.preview.title}>
      <div>
        <h3 className="text-sm font-semibold text-fg">{askCopy.preview.title}</h3>
        <p className="text-sm text-fg-muted">{askCopy.preview.body}</p>
      </div>
      <div className="flex items-end gap-2">
        <MeetingPicker
          meetings={meetings}
          value={chosen}
          onChange={(id) => {
            setMeeting(meetings.find((m) => m.id === id)?.id);
          }}
        />
        <Button
          variant="secondary"
          disabled={chosen === undefined || preview.busy}
          onClick={() => {
            if (chosen !== undefined) preview.run({ meeting: chosen, recipe: recipe.id });
          }}
        >
          <Play {...ICON_PROPS} />
          {preview.busy ? askCopy.preview.running : askCopy.preview.run}
        </Button>
      </div>
      <Result preview={preview} meeting={chosen} />
    </section>
  );
}

/** Which recent meeting to run on. */
function MeetingPicker({
  meetings,
  value,
  onChange,
}: {
  meetings: readonly Meeting[];
  value: MeetingId | undefined;
  onChange: (id: string) => void;
}) {
  return (
    <div className="grid min-w-0 flex-1 gap-1.5">
      <Label htmlFor="preview-meeting">{askCopy.preview.meeting}</Label>
      <Select value={value ?? ''} onValueChange={onChange}>
        <SelectTrigger id="preview-meeting">
          <SelectValue placeholder={askCopy.preview.choose} />
        </SelectTrigger>
        <SelectContent>
          {meetings.map((m) => (
            <SelectItem key={m.id} value={m.id}>
              {m.title}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}

/** The streaming text, the answer with its citations, or the error. */
function Result({
  preview,
  meeting,
}: {
  preview: ReturnType<typeof usePreview>;
  meeting: MeetingId | undefined;
}) {
  if (preview.error !== null) {
    return (
      <p role="alert" className="text-sm text-danger">
        {preview.error.message}
      </p>
    );
  }
  if (preview.busy) {
    const text = streamingText(preview.streamed);
    return (
      <p
        className="rounded-lg bg-bg-subtle px-3 py-2.5 text-sm whitespace-pre-wrap text-fg"
        aria-busy="true"
      >
        {text === '' ? askCopy.thinking : text}
      </p>
    );
  }
  if (preview.answer === undefined || meeting === undefined) return null;
  return (
    <div className="rounded-lg bg-bg-subtle px-3 py-2.5">
      <AnswerBody meetingId={preview.answer.meetingId} parts={preview.answer.parts} />
    </div>
  );
}

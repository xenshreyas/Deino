/**
 * The Notes tab before a summary exists: the house empty state with one
 * action (generate), the last error inline, and a hint when no model is set.
 */
import { Button } from '@/ui/button';
import { EmptyState } from '@/ui/empty-state';
import { ICON_PROPS, Sparkles } from '@/ui/icons';

/** Props of {@link EmptyStateSummary}. */
interface EmptyStateSummaryProps {
  onGenerate: () => void;
  hasModel: boolean;
  isGenerating?: boolean;
  error?: string | null;
}

/** The label of the one action, which changes after a failure. */
function actionLabel(isGenerating: boolean, error: string | null): string {
  if (isGenerating) return 'Generating…';
  return error === null ? 'Generate notes' : 'Retry summary';
}

/** "No notes yet" with Generate, the error and the model hint. */
export function EmptyStateSummary({
  onGenerate,
  hasModel,
  isGenerating = false,
  error = null,
}: EmptyStateSummaryProps) {
  return (
    <div className="flex flex-col items-center pb-12">
      <EmptyState
        className="pt-[12vh]"
        icon={Sparkles}
        title="No notes yet"
        description="Deino writes the key points, decisions and action items from the transcript."
        action={
          <Button variant="secondary" onClick={onGenerate} disabled={!hasModel || isGenerating}>
            <Sparkles {...ICON_PROPS} />
            {actionLabel(isGenerating, error)}
          </Button>
        }
      />
      {error === null ? null : (
        <p role="alert" className="mt-4 max-w-md rounded-md bg-danger-soft px-3 py-2 text-sm text-danger">
          {error}
        </p>
      )}
      {hasModel ? null : (
        <p className="mt-3 text-xs text-warning-text">Choose a summary model in Settings first.</p>
      )}
    </div>
  );
}

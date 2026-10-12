/**
 * The year in review (ORG-09). In December (or with the debug flag) the
 * page offers to write it; once written it is kept and shown as AI text.
 * Outside December it is one muted line at the foot of the page.
 */
import Markdown from 'react-markdown';
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import { ICON_PROPS, Sparkles } from '@/ui/icons';

import { statsCopy } from '../copy';
import { formatDate } from '../format';
import { useGenerateRecap } from '../queries';
import type { RecapStatus, YearRecap } from '../types';
import { StatSection } from './stat-section';

function RecapText({ recap }: { recap: YearRecap }) {
  return (
    <div className="flex flex-col gap-2">
      <div className="max-w-[45rem] text-base leading-[26px] text-pretty text-ai-text [&_h3]:mt-4 [&_h3]:mb-1 [&_h3]:text-sm [&_h3]:font-semibold [&_h3]:text-fg [&_li]:my-1 [&_p]:my-2 [&_strong]:font-semibold [&_strong]:text-fg [&_ul]:list-disc [&_ul]:pl-5">
        <Markdown>{recap.markdown}</Markdown>
      </div>
      <p className="text-xs text-fg-subtle">
        {statsCopy.recapWritten(formatDate(recap.generatedAt))}
      </p>
    </div>
  );
}

function GeneratePrompt() {
  const generate = useGenerateRecap();
  const write = () => {
    generate.mutate(undefined, {
      onError: (error) => toast.error(statsCopy.recapFailed, { description: error.message }),
    });
  };
  return (
    <div className="flex flex-col items-start gap-3">
      <p className="text-sm text-pretty text-fg-muted">{statsCopy.recapPitch}</p>
      <Button onClick={write} disabled={generate.isPending}>
        <Sparkles {...ICON_PROPS} />
        {generate.isPending ? statsCopy.recapGenerating : statsCopy.recapGenerate}
      </Button>
    </div>
  );
}

/** Props of {@link YearRecapSection}. */
export interface YearRecapSectionProps {
  status: RecapStatus;
}

/** The recap, the offer to write it, or nothing until December. */
export function YearRecapSection({ status }: YearRecapSectionProps) {
  if (status.recap === null && !status.available) return null;
  return (
    <StatSection id="stats-recap" title={statsCopy.recap(status.year)}>
      {status.recap === null ? <GeneratePrompt /> : <RecapText recap={status.recap} />}
    </StatSection>
  );
}

/** The foot-of-page line before December. */
export function RecapClosedLine({ status }: YearRecapSectionProps) {
  if (status.recap !== null || status.available) return null;
  return <p className="text-xs text-fg-subtle">{statsCopy.recapClosed(status.year)}</p>;
}

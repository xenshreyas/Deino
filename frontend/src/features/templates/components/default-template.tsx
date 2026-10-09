import { usePatchSettings, useSettings } from '@/features/settings';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/ui/select';

import { templatesCopy } from '../copy';
import type { Template } from '../types';

/** The menu value standing for "Auto" (no default template). */
const AUTO = 'auto';

/** Settings → Templates: the default template, or Auto. */
export function DefaultTemplate({ templates }: { templates: readonly Template[] }) {
  const settings = useSettings();
  const patch = usePatchSettings();
  const value = settings.data?.templates.defaultTemplate ?? AUTO;
  return (
    <div className="flex items-center justify-between gap-6 rounded-lg border border-border bg-surface px-4 py-3">
      <div className="flex min-w-0 flex-col gap-0.5">
        <span className="text-sm text-fg">{templatesCopy.defaultLabel}</span>
        <span className="text-xs text-fg-muted">{templatesCopy.defaultDescription}</span>
      </div>
      <Select
        value={value}
        disabled={settings.data === undefined}
        onValueChange={(next) => {
          const chosen = templates.find((template) => template.id === next)?.id ?? null;
          patch.mutate({ templates: { defaultTemplate: chosen } });
        }}
      >
        <SelectTrigger aria-label={templatesCopy.defaultLabel} className="w-[200px] shrink-0">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={AUTO}>{templatesCopy.auto}</SelectItem>
          {templates.map((template) => (
            <SelectItem key={template.id} value={template.id}>
              {template.content.name}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}

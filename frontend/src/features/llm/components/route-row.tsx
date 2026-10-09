import { Button } from '@/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/ui/select';

import { llmCopy } from '../copy';
import { useModels, useRoutePin, useTestRoute } from '../queries';
import type { ProviderId, ProviderStatus, ResolvedRoute, RouteTestReport } from '../types';

/** Props of {@link RouteRow}. */
export interface RouteRowProps {
  row: ResolvedRoute;
  providers: readonly ProviderStatus[];
}

/** One task route: its provider and model pickers, a reset, and a test. */
export function RouteRow({ row, providers }: RouteRowProps) {
  const pinning = useRoutePin(row);
  const test = useTestRoute(row.route);
  const locked = row.route === 'embed';
  return (
    <li className="flex flex-col gap-2 py-3">
      <RouteHeading row={row} onReset={pinning.reset} />
      <div className="flex items-center gap-2">
        <ProviderPicker
          row={row}
          providers={providers}
          disabled={locked}
          onChange={pinning.chooseProvider}
        />
        <ModelPicker row={row} disabled={locked} onChange={pinning.chooseModel} />
        <Button
          variant="secondary"
          size="sm"
          disabled={test.isPending}
          onClick={() => {
            test.mutate();
          }}
        >
          {test.isPending ? llmCopy.testing : llmCopy.test}
        </Button>
      </div>
      <TestOutcome reply={test.reply} report={test.data} error={test.error} />
      {pinning.error === null ? null : (
        <p role="alert" className="text-sm text-danger">
          {pinning.error.message}
        </p>
      )}
    </li>
  );
}

/** The route's name, tier and source, and a reset for a custom choice. */
function RouteHeading({ row, onReset }: { row: ResolvedRoute; onReset: () => void }) {
  return (
    <div className="flex items-center justify-between gap-3">
      <span className="flex flex-col">
        <span className="text-sm text-fg">{llmCopy.routes[row.route]}</span>
        <span className="text-xs text-fg-muted">
          {llmCopy.tiers[row.tier]} · {row.source === 'custom' ? llmCopy.custom : llmCopy.default}
        </span>
      </span>
      {row.source === 'custom' ? (
        <Button variant="ghost" size="sm" onClick={onReset}>
          {llmCopy.reset}
        </Button>
      ) : null}
    </div>
  );
}

interface PickerProps<T> {
  row: ResolvedRoute;
  disabled: boolean;
  onChange: (value: T) => void;
}

/** Every provider; choosing one pins the route to it. */
function ProviderPicker({
  row,
  providers,
  disabled,
  onChange,
}: PickerProps<ProviderId> & { providers: readonly ProviderStatus[] }) {
  return (
    <Select value={row.target.provider} disabled={disabled} onValueChange={onChange}>
      <SelectTrigger
        aria-label={`${llmCopy.providerColumn}: ${llmCopy.routes[row.route]}`}
        className="w-44"
      >
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {providers.map((provider) => (
          <SelectItem key={provider.id} value={provider.id}>
            {provider.name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

/** The provider's live model list; the current model stays listed even if it is not. */
function ModelPicker({ row, disabled, onChange }: PickerProps<string>) {
  const models = useModels(disabled ? null : row.target.provider);
  const ids = (models.data ?? []).map((model) => model.id);
  const options = ids.includes(row.target.model) ? ids : [row.target.model, ...ids];
  const placeholder = models.isLoading ? llmCopy.loadingModels : llmCopy.modelsFailed;
  return (
    <Select value={row.target.model} disabled={disabled} onValueChange={onChange}>
      <SelectTrigger
        aria-label={`${llmCopy.modelColumn}: ${llmCopy.routes[row.route]}`}
        className="min-w-0 flex-1"
      >
        <SelectValue placeholder={placeholder} />
      </SelectTrigger>
      <SelectContent>
        {options.map((model) => (
          <SelectItem key={model} value={model}>
            {model}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

interface TestOutcomeProps {
  reply: string;
  report: RouteTestReport | undefined;
  error: Error | null;
}

/** The streamed test reply, then which model answered and how fast. */
function TestOutcome({ reply, report, error }: TestOutcomeProps) {
  if (error !== null) {
    return (
      <p role="alert" className="text-xs text-danger">
        {error.message}
      </p>
    );
  }
  const summary =
    report === undefined
      ? null
      : report.dimensions === null
        ? llmCopy.testReport(report.target.model, report.latencyMs)
        : llmCopy.embedReport(report.dimensions);
  if (reply === '' && summary === null) return null;
  return (
    <p role="status" className="rounded-md bg-bg-subtle px-3 py-2 text-xs text-fg-muted">
      {reply}
      {summary === null ? null : <span className="mt-1 block text-fg-subtle">{summary}</span>}
    </p>
  );
}

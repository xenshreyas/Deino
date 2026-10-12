/**
 * LLM usage per provider and model, from the gateway's `llm_calls` log:
 * calls, tokens and the estimated cost, with the estimate stated plainly
 * under the table. Numbers align in tabular columns.
 */
import { statsCopy } from '../copy';
import { formatCost, formatCount } from '../format';
import type { LlmUsage, ModelUsage } from '../types';
import { RestingLine } from './stat-section';

/** Providers that run on the Mac. */
const LOCAL = new Set<ModelUsage['provider']>(['ollama', 'builtin']);

function costOf(model: ModelUsage): string {
  if (model.estimatedCostMicros === null) return '—';
  if (LOCAL.has(model.provider)) return statsCopy.local;
  return formatCost(model.estimatedCostMicros);
}

function Row({ model }: { model: ModelUsage }) {
  return (
    <tr className="border-t border-border">
      <td className="py-2 pr-4">
        <div className="truncate text-fg">{model.model}</div>
        <div className="text-xs text-fg-subtle">{statsCopy.providers[model.provider]}</div>
      </td>
      <td className="py-2 pr-4 text-right tabular-nums">
        {formatCount(model.calls)}
        {model.failed === 0 ? null : (
          <div className="text-xs text-fg-subtle">{statsCopy.failed(model.failed)}</div>
        )}
      </td>
      <td className="py-2 pr-4 text-right text-fg-muted tabular-nums">
        {formatCount(model.inputTokens)} / {formatCount(model.outputTokens)}
      </td>
      <td className="py-2 text-right tabular-nums">{costOf(model)}</td>
    </tr>
  );
}

/** Props of {@link LlmUsageTable}. */
export interface LlmUsageTableProps {
  usage: LlmUsage;
}

/** One row per model, then the estimate's caveats. */
export function LlmUsageTable({ usage }: LlmUsageTableProps) {
  if (usage.models.length === 0) return <RestingLine>{statsCopy.noLlm}</RestingLine>;
  return (
    <div className="flex flex-col gap-2">
      <table className="w-full table-fixed text-sm">
        <thead>
          <tr className="text-left text-xs text-fg-muted">
            <th className="w-[46%] pb-2 font-normal">{statsCopy.model}</th>
            <th className="pr-4 pb-2 text-right font-normal">{statsCopy.calls}</th>
            <th className="pr-4 pb-2 text-right font-normal">{statsCopy.tokens}</th>
            <th className="pb-2 text-right font-normal">{statsCopy.estCost}</th>
          </tr>
        </thead>
        <tbody>
          {usage.models.map((model) => (
            <Row key={`${model.provider}/${model.model}`} model={model} />
          ))}
        </tbody>
      </table>
      <p className="text-xs text-pretty text-fg-subtle">
        {statsCopy.llmEstimate}
        {usage.unpricedCalls === 0 ? null : ` ${statsCopy.llmUnpriced(usage.unpricedCalls)}`}
      </p>
    </div>
  );
}

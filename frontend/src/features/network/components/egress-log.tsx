/**
 * The egress log on the Network page: every request Deino made, newest
 * first, with when, which service, which host and how many bytes went each
 * way. It never shows a path, a query, a header or a body; Rust never
 * records them.
 */
import { cn } from '@/ui/cn';
import { SkeletonText } from '@/ui/skeleton';

import { networkCopy } from '../copy';
import { formatBytes, formatTime } from '../format';
import { useEgressLog } from '../queries';
import type { EgressEntry } from '../types';
import { SettingsSection } from './settings-section';

/** What happened to a request: its status, or why it never got one. */
function outcomeLabel(entry: EgressEntry): string {
  if (entry.outcome === 'sent' && entry.status !== null) {
    return `${entry.method} ${String(entry.status)}`;
  }
  return `${entry.method} · ${networkCopy.outcomes[entry.outcome]}`;
}

/** One request. */
function EgressRow({ entry }: { entry: EgressEntry }) {
  const columns = networkCopy.log.columns;
  return (
    <tr className="align-baseline">
      <td className="py-1.5 pr-3 whitespace-nowrap text-fg-muted tabular-nums">
        <span className="sr-only">{columns.time}: </span>
        {formatTime(entry.at)}
      </td>
      <td className="py-1.5 pr-3 text-fg">{networkCopy.serviceNames[entry.service]}</td>
      <td className="max-w-[180px] truncate py-1.5 pr-3 font-mono text-fg-muted" title={entry.host}>
        {entry.host}
      </td>
      <td
        className={cn(
          'py-1.5 pr-3 whitespace-nowrap',
          entry.outcome === 'sent' ? 'text-fg-muted' : 'text-danger',
        )}
      >
        {outcomeLabel(entry)}
      </td>
      <td className="py-1.5 text-right whitespace-nowrap text-fg-muted tabular-nums">
        {formatBytes(entry.bytesUp)} / {formatBytes(entry.bytesDown)}
      </td>
    </tr>
  );
}

/** Props of {@link EgressLogTable}. */
export interface EgressLogTableProps {
  entries: readonly EgressEntry[];
}

/** The requests as a compact table, or a calm line when there are none. */
export function EgressLogTable({ entries }: EgressLogTableProps) {
  if (entries.length === 0) {
    return <p className="py-2 text-sm text-fg-muted">{networkCopy.log.empty}</p>;
  }
  const columns = networkCopy.log.columns;
  return (
    <div className="rounded-lg border border-border bg-surface px-4 py-2">
      <table className="w-full text-xs">
        <thead className="sr-only">
          <tr>
            <th>{columns.time}</th>
            <th>{columns.service}</th>
            <th>{columns.host}</th>
            <th>{networkCopy.outcomes.sent}</th>
            <th>{columns.size}</th>
          </tr>
        </thead>
        <tbody>
          {entries.map((entry, index) => (
            <EgressRow key={`${entry.at}-${String(index)}`} entry={entry} />
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** The activity section, kept fresh by `DbChanged` after every request. */
export function EgressLogSection() {
  const log = useEgressLog();
  return (
    <SettingsSection title={networkCopy.log.title} description={networkCopy.log.description}>
      {log.isPending ? <SkeletonText lines={3} /> : null}
      {log.isError ? (
        <p role="alert" className="text-sm text-danger">
          {networkCopy.log.error}
        </p>
      ) : null}
      {log.data === undefined ? null : <EgressLogTable entries={log.data} />}
    </SettingsSection>
  );
}

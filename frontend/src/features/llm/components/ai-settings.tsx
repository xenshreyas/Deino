import { Separator } from '@/ui/controls';
import { SkeletonText } from '@/ui/skeleton';

import { llmCopy } from '../copy';
import { useLlmSettingsChanges, useProviders, useRoutes } from '../queries';
import { ProviderList } from './provider-list';
import { RouteRow } from './route-row';

/** The per-route model table. */
function RouteTable() {
  const routes = useRoutes();
  const providers = useProviders();
  return (
    <section aria-labelledby="llm-routes" className="flex flex-col gap-2">
      <h3 id="llm-routes" className="text-sm font-semibold text-fg">
        {llmCopy.routesTitle}
      </h3>
      <p className="text-xs text-fg-muted">{llmCopy.routesDescription}</p>
      {routes.data === undefined || providers.data === undefined ? (
        <SkeletonText lines={6} />
      ) : (
        <ul className="divide-y divide-border">
          {routes.data.map((row) => (
            <RouteRow key={row.route} row={row} providers={providers.data} />
          ))}
        </ul>
      )}
      {routes.error === null ? null : (
        <p role="alert" className="text-sm text-danger">
          {routes.error.message}
        </p>
      )}
    </section>
  );
}

/**
 * Settings → AI: the providers Deino can use (masked keys only) and which
 * provider and model serve each AI task, with a Test button per task.
 */
export function AiSettings() {
  useLlmSettingsChanges();
  return (
    <div className="flex flex-col gap-6">
      <ProviderList />
      <Separator />
      <RouteTable />
    </div>
  );
}

import { CircleCheck, ICON_PROPS } from '@/ui/icons';
import { SkeletonText } from '@/ui/skeleton';

import { llmCopy } from '../copy';
import { useProviders } from '../queries';
import type { ProviderStatus } from '../types';

/** One provider: its name, masked key and server address. */
function ProviderRow({ provider }: { provider: ProviderStatus }) {
  const key =
    provider.maskedKey ?? (provider.requiresKey ? llmCopy.keyMissing : llmCopy.keyNotNeeded);
  return (
    <li className="flex items-center justify-between gap-4 py-2">
      <span className="flex items-center gap-2 text-sm text-fg">
        {provider.maskedKey === null ? null : (
          <CircleCheck {...ICON_PROPS} className="text-accent" />
        )}
        {provider.name}
      </span>
      <span className="truncate font-mono text-xs text-fg-muted">{provider.endpoint ?? key}</span>
    </li>
  );
}

/** Settings → AI: every provider with its masked key, never the key itself. */
export function ProviderList() {
  const providers = useProviders();
  return (
    <section aria-labelledby="llm-providers" className="flex flex-col gap-2">
      <h4 id="llm-providers" className="text-sm font-semibold text-fg">
        {llmCopy.providersTitle}
      </h4>
      <p className="text-xs text-fg-muted">{llmCopy.providersDescription}</p>
      {providers.data === undefined ? (
        <SkeletonText lines={3} />
      ) : (
        <ul className="divide-y divide-border">
          {providers.data.map((provider) => (
            <ProviderRow key={provider.id} provider={provider} />
          ))}
        </ul>
      )}
      {providers.error === null ? null : (
        <p role="alert" className="text-sm text-danger">
          {providers.error.message}
        </p>
      )}
    </section>
  );
}

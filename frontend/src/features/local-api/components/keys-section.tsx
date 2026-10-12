import { SettingsCard, SettingsSection } from '@/ui/settings-section';
import { Skeleton } from '@/ui/skeleton';

import { apiCopy } from '../copy';
import { useApiKeys, useCreateApiKey, useDeleteApiKey, useRevokeApiKey } from '../queries';
import type { ApiKey } from '../types';
import { KeyRow } from './key-row';
import { NewKeyForm, NewKeyReveal } from './new-key';

/** Settings → Developer's API keys: make one, see its token once, revoke it (container). */
export function KeysSection() {
  const create = useCreateApiKey();
  return (
    <SettingsSection title={apiCopy.keysTitle} description={apiCopy.keysDescription}>
      <SettingsCard label={apiCopy.keysTitle}>
        {create.data === undefined ? (
          <NewKeyForm create={create} />
        ) : (
          <NewKeyReveal
            created={create.data}
            onDone={() => {
              create.reset();
            }}
          />
        )}
        <KeyList />
      </SettingsCard>
    </SettingsSection>
  );
}

/** Every key, newest first. */
function KeyList() {
  const keys = useApiKeys();
  const revoke = useRevokeApiKey();
  const remove = useDeleteApiKey();
  if (keys.isPending) return <Skeleton className="m-4 h-10" />;
  if (keys.isError) {
    return (
      <p role="alert" className="px-4 py-3 text-sm text-danger">
        {apiCopy.keysFailed}
      </p>
    );
  }
  if (keys.data.length === 0) {
    return <p className="px-4 py-3 text-sm text-fg-muted">{apiCopy.noKeys}</p>;
  }
  const busy = revoke.isPending || remove.isPending;
  return keys.data.map((key: ApiKey) => (
    <KeyRow
      key={key.id}
      apiKey={key}
      busy={busy}
      onRevoke={() => {
        revoke.mutate(key.id);
      }}
      onRemove={() => {
        remove.mutate(key.id);
      }}
    />
  ));
}

/**
 * One shipped provider on Settings → Integrations: its name and what it
 * does, Connect (a token form inline, or a browser sign-in with Cancel),
 * and its connected accounts. Owns its mutations; the list refreshes
 * through `DbChanged`.
 */
import { useState } from 'react';

import { IpcError } from '@/lib/ipc/result';
import { Button } from '@/ui/button';
import { SettingsCard } from '@/ui/settings-section';

import { integrationsCopy } from '../copy';
import { capabilitySummary, hasOAuth, tokenMethod } from '../format';
import {
  useCancelConnect,
  useCheckConnection,
  useConnect,
  useConnectToken,
  useDisconnect,
} from '../queries';
import type { ProviderView } from '../types';
import { ConnectionRow } from './connection-row';
import { TokenForm, type TokenFields } from './token-form';

const copy = integrationsCopy.provider;

/** Props of {@link ProviderCard}. */
export interface ProviderCardProps {
  provider: ProviderView;
  now: Date;
}

/** What the header offers on its right: Connect, Cancel (while the browser
 * sign-in waits) or nothing (connected, or the token form is open). */
type HeaderControl = 'connect' | 'cancel' | null;

/** The name, what the provider does and its control. */
function Header({
  provider,
  control,
  onConnect,
  onCancel,
}: {
  provider: ProviderView;
  control: HeaderControl;
  onConnect: () => void;
  onCancel: () => void;
}) {
  const waiting = control === 'cancel';
  const accounts = provider.connections.length;
  const state = accounts === 0 ? copy.notConnected : copy.connectedAs(accounts);
  return (
    <div className="grid min-h-[52px] grid-cols-[1fr_auto] items-center gap-4 px-4 py-3">
      <div className="min-w-0">
        <p className="text-sm text-fg">{provider.name}</p>
        <p className="truncate text-xs text-fg-muted">
          {waiting ? copy.waiting : `${state} · ${capabilitySummary(provider.capabilities)}`}
        </p>
      </div>
      {waiting ? (
        <Button variant="ghost" size="sm" onClick={onCancel}>
          {copy.cancel}
        </Button>
      ) : null}
      {control === 'connect' ? (
        <Button size="sm" onClick={onConnect}>
          {copy.connect}
        </Button>
      ) : null}
    </div>
  );
}

/** The header control for the card's state. */
function headerControl(waiting: boolean, formOpen: boolean, provider: ProviderView): HeaderControl {
  if (waiting) return 'cancel';
  return formOpen || provider.connections.length > 0 ? null : 'connect';
}

/** What to say about a failed connect; nothing when the user cancelled. */
function visibleError(error: Error | null): string | null {
  if (error === null) return null;
  const failure = IpcError.from(error);
  return failure.code === 'cancelled' ? null : failure.message;
}

/** The connect mutations of one provider, as the card uses them. */
function useConnecting(provider: ProviderView) {
  const [formOpen, setFormOpen] = useState(false);
  const token = useConnectToken();
  const browser = useConnect();
  const cancel = useCancelConnect();
  const method = tokenMethod(provider.methods);
  const start = () => {
    if (method !== undefined) setFormOpen(true);
    else if (hasOAuth(provider.methods)) browser.mutate(provider.id);
  };
  const submit = ({ token: value, username }: TokenFields) => {
    token.mutate(
      { provider: provider.id, token: value, username },
      {
        onSuccess: () => {
          setFormOpen(false);
        },
      },
    );
  };
  const error = token.error ?? browser.error;
  return {
    method,
    formOpen: formOpen && method !== undefined,
    closeForm: () => {
      setFormOpen(false);
      token.reset();
    },
    start,
    submit,
    waiting: browser.isPending,
    busy: token.isPending,
    cancel: () => {
      cancel.mutate(provider.id);
    },
    error: visibleError(error),
  };
}

/** The provider's accounts, each with Check and Disconnect. */
function Connections({ provider, now }: ProviderCardProps) {
  const check = useCheckConnection();
  const disconnect = useDisconnect();
  const busyId = check.isPending
    ? check.variables
    : disconnect.isPending
      ? disconnect.variables
      : null;
  return provider.connections.map((connection) => (
    <ConnectionRow
      key={connection.id}
      connection={connection}
      now={now}
      busy={busyId === connection.id}
      onCheck={() => {
        check.mutate(connection.id);
      }}
      onDisconnect={() => {
        disconnect.mutate(connection.id);
      }}
    />
  ));
}

/** A shipped provider with its accounts. */
export function ProviderCard({ provider, now }: ProviderCardProps) {
  const connecting = useConnecting(provider);
  return (
    <SettingsCard label={provider.name}>
      <Header
        provider={provider}
        control={headerControl(connecting.waiting, connecting.formOpen, provider)}
        onConnect={connecting.start}
        onCancel={connecting.cancel}
      />
      {connecting.formOpen && connecting.method !== undefined ? (
        <TokenForm
          method={connecting.method}
          busy={connecting.busy}
          error={connecting.error}
          onSubmit={connecting.submit}
          onCancel={connecting.closeForm}
        />
      ) : null}
      {!connecting.formOpen && connecting.error !== null ? (
        <p role="alert" className="px-4 py-2 text-xs text-danger">
          {connecting.error}
        </p>
      ) : null}
      <Connections provider={provider} now={now} />
    </SettingsCard>
  );
}

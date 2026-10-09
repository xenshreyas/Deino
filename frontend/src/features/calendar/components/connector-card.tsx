/**
 * One direct connection of Settings → Calendar (Google Calendar or
 * Microsoft Outlook): its state with Connect or Disconnect, and, until it is
 * connected, the user's app registration (client ID, and Google's client
 * secret, which goes to the Keychain). Presentational.
 */
import type { ReactNode } from 'react';

import type { SecretStatus } from '@/features/settings';
import { Button } from '@/ui/button';
import { Input } from '@/ui/input';
import { SettingsCard } from '@/ui/settings-section';

import { calendarCopy } from '../copy';
import { syncedLabel } from '../synced';
import type { ConnectorStatus } from '../types';

const copy = calendarCopy.connectors;

/** What the user can do with a connection. */
export interface ConnectorActions {
  onSaveClientId: (clientId: string) => void;
  onSaveSecret: (secret: string) => void;
  onConnect: () => void;
  onDisconnect: () => void;
}

/** Props of {@link ConnectorCard}. */
export interface ConnectorCardProps {
  connector: ConnectorStatus;
  /** The saved client ID; empty when none is set. */
  clientId: string;
  /** The client secret's Keychain status; `null` for a provider without one. */
  secret: { status: SecretStatus | undefined } | null;
  now: Date;
  /** A sign-in or disconnect of this provider is in flight. */
  busy: boolean;
  actions: ConnectorActions;
}

/** Whether everything the sign-in needs is saved. */
function isReady({ connector, secret }: Pick<ConnectorCardProps, 'connector' | 'secret'>) {
  return connector.configured && (secret === null || secret.status === 'stored');
}

/** The line under the name. */
function summary(props: ConnectorCardProps): string {
  const { connector, now, busy } = props;
  if (connector.connected) return copy.connected(syncedLabel(connector.lastSyncedAt, now));
  if (busy) return copy.waiting;
  return isReady(props) ? copy.notConnected : copy.needsSetup;
}

/** The name, the state and the Connect or Disconnect button. */
function HeaderRow(props: ConnectorCardProps) {
  const { connector, busy, actions } = props;
  const name = copy.names[connector.provider];
  return (
    <div className="grid min-h-[52px] grid-cols-[1fr_auto] items-center gap-4 px-4 py-3">
      <div className="min-w-0">
        <p className="text-sm text-fg">{name}</p>
        <p className="text-xs text-fg-muted">{summary(props)}</p>
      </div>
      {connector.connected ? (
        <Button variant="secondary" size="sm" disabled={busy} onClick={actions.onDisconnect}>
          {copy.disconnect}
        </Button>
      ) : (
        <Button size="sm" disabled={busy || !isReady(props)} onClick={actions.onConnect}>
          {copy.connect}
        </Button>
      )}
    </div>
  );
}

/** Props of {@link FieldRow}. */
interface FieldRowProps {
  id: string;
  label: string;
  children: ReactNode;
}

/** A labelled field on its own row. */
function FieldRow({ id, label, children }: FieldRowProps) {
  return (
    <div className="flex flex-col gap-1 px-4 py-3">
      <label htmlFor={id} className="text-xs text-fg-muted">
        {label}
      </label>
      {children}
    </div>
  );
}

/** The client ID, saved when the field loses focus. */
function ClientIdField({ connector, clientId, busy, actions }: ConnectorCardProps) {
  const id = `calendar-${connector.provider}-client-id`;
  return (
    <FieldRow id={id} label={copy.clientId}>
      <Input
        id={id}
        key={clientId}
        defaultValue={clientId}
        placeholder={copy.clientIdPlaceholder}
        autoComplete="off"
        spellCheck={false}
        disabled={busy}
        onBlur={(event) => {
          const next = event.currentTarget.value.trim();
          if (next !== clientId) actions.onSaveClientId(next);
        }}
      />
    </FieldRow>
  );
}

/** The client secret: write-only, saved to the Keychain on blur. */
function SecretField({ connector, secret, busy, actions }: ConnectorCardProps) {
  const id = `calendar-${connector.provider}-client-secret`;
  const stored = secret?.status === 'stored';
  return (
    <FieldRow id={id} label={copy.clientSecret}>
      <Input
        id={id}
        type="password"
        placeholder={stored ? copy.secretStored : copy.secretMissing}
        autoComplete="off"
        disabled={busy}
        onBlur={(event) => {
          const value = event.currentTarget.value.trim();
          if (value === '') return;
          actions.onSaveSecret(value);
          event.currentTarget.value = '';
        }}
      />
    </FieldRow>
  );
}

/** A direct connection's card. */
export function ConnectorCard(props: ConnectorCardProps) {
  const { connector, secret } = props;
  return (
    <SettingsCard label={copy.names[connector.provider]}>
      <HeaderRow {...props} />
      {connector.connected ? null : (
        <>
          <p className="px-4 py-2 text-xs text-fg-muted">{copy.setup[connector.provider]}</p>
          <ClientIdField {...props} />
          {secret === null ? null : <SecretField {...props} />}
        </>
      )}
      {connector.lastError === null ? null : (
        <p role="alert" className="px-4 py-2 text-xs text-danger">
          {connector.lastError}
        </p>
      )}
    </SettingsCard>
  );
}

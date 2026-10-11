/**
 * Settings → Calendar → Direct connections (ORG-01): the optional Google
 * Calendar and Microsoft Graph connectors. Container: the connection state
 * comes from the calendar status, the app registration from the settings
 * document and the Keychain.
 */
import { useQueryClient } from '@tanstack/react-query';

import {
  type SecretName,
  usePatchSettings,
  useSaveSecret,
  useSecretStatus,
  useSettings,
} from '@/features/settings';
import { SettingsSection } from '@/ui/settings-section';

import { calendarCopy } from '../copy';
import { useNow } from '../hooks/use-now';
import { calendarKeys, useCalendarStatus, useConnectCalendar } from '../queries';
import type { CalendarProvider, ConnectorStatus } from '../types';
import { ConnectorCard, type ConnectorActions } from './connector-card';

const copy = calendarCopy.connectors;

/** The providers with a direct connector. */
type DirectProvider = Exclude<CalendarProvider, 'apple'>;

/** The Keychain secret each provider's sign-in needs besides its client ID. */
const CLIENT_SECRETS: Record<DirectProvider, SecretName | null> = {
  google: 'google_oauth_client_secret',
  microsoft: null,
};

/** A direct connector's status, narrowed to its provider. */
type DirectConnector = ConnectorStatus & { provider: DirectProvider };

function isDirect(connector: ConnectorStatus): connector is DirectConnector {
  return connector.provider !== 'apple';
}

/** Saving the app registration: the client ID and the client secret. */
function useRegistration(provider: DirectProvider) {
  const client = useQueryClient();
  const settings = useSettings();
  const patch = usePatchSettings();
  const saveSecret = useSaveSecret();
  const secretName = CLIENT_SECRETS[provider];
  const secretStatus = useSecretStatus(secretName);
  return {
    clientId: settings.data?.calendar[provider].clientId ?? '',
    secret: secretName === null ? null : { status: secretStatus.data },
    saveClientId: (clientId: string) => {
      // The status says whether a connector is set up: read it again.
      patch.mutate(
        { calendar: { [provider]: { clientId } } },
        { onSuccess: () => void client.invalidateQueries({ queryKey: calendarKeys.status() }) },
      );
    },
    saveSecret: (value: string) => {
      if (secretName !== null) saveSecret.mutate({ name: secretName, value });
    },
    failed: patch.isError || saveSecret.isError,
  };
}

/** The registration, the actions and their state for one connector. */
function useConnector(provider: DirectProvider) {
  const { saveClientId, saveSecret, failed, ...registration } = useRegistration(provider);
  const change = useConnectCalendar();
  const actions: ConnectorActions = {
    onSaveClientId: saveClientId,
    onSaveSecret: saveSecret,
    onConnect: () => {
      change.mutate({ provider, connected: true });
    },
    onDisconnect: () => {
      change.mutate({ provider, connected: false });
    },
  };
  let failure: string | null = failed ? copy.saveFailed : null;
  if (change.isError) {
    failure = change.variables.connected ? copy.connectFailed : copy.disconnectFailed;
  }
  return { ...registration, busy: change.isPending, actions, failure };
}

/** One connector's card and what went wrong with the last action. */
function ConnectorItem({ connector, now }: { connector: DirectConnector; now: Date }) {
  const { failure, ...state } = useConnector(connector.provider);
  return (
    <div className="flex flex-col gap-1">
      <ConnectorCard connector={connector} now={now} {...state} />
      {failure === null ? null : (
        <p role="alert" className="text-xs text-danger">
          {failure}
        </p>
      )}
    </div>
  );
}

/** The direct connections; nothing when the app has no connectors. */
export function ConnectorsSection() {
  const now = useNow();
  const status = useCalendarStatus();
  const connectors = status.data?.connectors.filter(isDirect) ?? [];
  if (connectors.length === 0) return null;
  return (
    <SettingsSection title={copy.title} description={copy.description}>
      {connectors.map((connector) => (
        <ConnectorItem key={connector.provider} connector={connector} now={now} />
      ))}
    </SettingsSection>
  );
}

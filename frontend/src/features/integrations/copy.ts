import type { Capability, ConnectionStatus, DeliveryStatus, IntegrationProvider } from './types';

/** Every visible string of the integrations feature. */
export const integrationsCopy = {
  intro:
    'Each integration talks straight from this Mac to the service, with your own account. Sign-ins stay in the Keychain.',
  offline: 'Offline mode is on, so integrations send nothing until you turn it off in Network.',
  calendars: {
    text: 'Google and Outlook calendars connect in Calendar.',
    link: 'Open Calendar',
  },
  available: {
    title: 'Connected apps',
    description: 'Connect an account to send notes, tasks and messages to it.',
    loadError: 'Could not load the integrations. Close and reopen Settings to try again.',
  },
  planned: {
    title: 'On the way',
    description: 'These connect in a later update.',
    badge: 'WIP',
    badgeTitle: 'Work in progress: this integration is coming in a later update',
  },
  provider: {
    notConnected: 'Not connected',
    connectedAs: (count: number) => (count === 1 ? '1 account' : `${String(count)} accounts`),
    connect: 'Connect',
    connectToken: (label: string) => `Connect with ${label.toLowerCase()}`,
    connectBrowser: 'Connect in browser',
    waiting: 'Waiting for your browser…',
    cancel: 'Cancel',
    save: 'Connect',
    tokenHelp: 'Where do I find it?',
  },
  connection: {
    disconnect: 'Disconnect',
    check: 'Check',
    lastUsed: (when: string) => `Last used ${when}`,
    neverUsed: 'Not used yet',
    access: (scopes: string) => `Access: ${scopes}`,
    actionError: 'That did not work. Try again.',
  },
  statuses: {
    connected: 'Connected',
    needs_reconnect: 'Needs reconnecting',
  } satisfies Record<ConnectionStatus, string>,
  capabilities: {
    post_message: 'Posts messages',
    create_page: 'Creates pages',
    create_task: 'Creates tasks',
    log_activity: 'Logs meetings on contacts',
    read_context: 'Reads context on demand',
  } satisfies Record<Capability, string>,
  deliveries: {
    title: 'Recently sent',
    description: 'What Deino sent to your integrations, newest first.',
    empty: 'Nothing sent yet.',
    loadError: 'Could not load what was sent.',
    open: 'Open',
    attempts: (count: number) => (count === 1 ? '1 attempt' : `${String(count)} attempts`),
  },
  deliveryStatuses: {
    pending: 'Waiting',
    sending: 'Sending',
    succeeded: 'Sent',
    failed: 'Failed',
  } satisfies Record<DeliveryStatus, string>,
  justNow: 'just now',
  /** Mirrors `IntegrationProvider::name` in Rust, for rows that only carry the ID. */
  providerNames: {
    slack: 'Slack',
    notion: 'Notion',
    linear: 'Linear',
    todoist: 'Todoist',
    asana: 'Asana',
    jira: 'Jira',
    monday: 'monday.com',
    clickup: 'ClickUp',
    trello: 'Trello',
    hubspot: 'HubSpot',
    attio: 'Attio',
    salesforce: 'Salesforce',
    affinity: 'Affinity',
    zoho_crm: 'Zoho CRM',
    google_docs: 'Google Docs',
    gmail: 'Gmail',
    outlook_mail: 'Outlook',
    sandbox: 'Sandbox',
  } satisfies Record<IntegrationProvider, string>,
} as const;

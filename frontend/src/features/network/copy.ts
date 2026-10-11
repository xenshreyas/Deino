import type { EgressOutcome, ServiceId, ServiceKind } from './types';

/** Every visible string of the network feature. */
export const networkCopy = {
  offline: {
    title: 'Offline mode',
    description:
      'Send nothing from this Mac. Model downloads you start still work, and notes use the built-in model.',
  },
  services: {
    title: 'Services',
    description:
      'Deino talks only to these services, and only to their hosts. A service that is off sends nothing.',
    error: 'Could not load the services. Close and reopen Settings to try again.',
    switchError: 'Could not save that change. Try again.',
  },
  log: {
    title: 'Recent activity',
    description: 'Every request Deino made: when, where and how big. Never what it carried.',
    empty: 'Nothing has left this Mac yet.',
    error: 'Could not load the activity log.',
    columns: { time: 'Time', service: 'Service', host: 'Host', size: 'Sent / received' },
  },
  kinds: {
    llm: 'AI providers',
    models: 'Model downloads',
    integration: 'Connected apps',
    web: 'Web',
  } satisfies Record<ServiceKind, string>,
  outcomes: {
    sent: 'Sent',
    failed: 'Failed',
    blocked: 'Blocked',
  } satisfies Record<EgressOutcome, string>,
  serviceNames: {
    llm_anthropic: 'Anthropic',
    llm_openai: 'OpenAI',
    llm_groq: 'Groq',
    llm_openrouter: 'OpenRouter',
    llm_custom: 'Custom endpoint',
    llm_ollama: 'Ollama',
    hf_models: 'Hugging Face',
    google_calendar: 'Google Calendar',
    google_drive: 'Google Drive',
    gmail: 'Gmail',
    microsoft_graph: 'Microsoft 365',
    slack: 'Slack',
    notion: 'Notion',
    linear: 'Linear',
    todoist: 'Todoist',
    asana: 'Asana',
    hubspot: 'HubSpot',
    attio: 'Attio',
    salesforce: 'Salesforce',
    jira: 'Jira',
    monday: 'monday.com',
    clickup: 'ClickUp',
    trello: 'Trello',
    affinity: 'Affinity',
    zoho_crm: 'Zoho CRM',
    web_search: 'Web search',
    webhooks: 'Webhooks',
    favicons: 'Company logos',
  } satisfies Record<ServiceId, string>,
  noHosts: 'Set up in Settings first',
} as const;

/** The accessible name of a service's switch. */
export function switchLabel(service: ServiceId): string {
  return `Allow ${networkCopy.serviceNames[service]}`;
}

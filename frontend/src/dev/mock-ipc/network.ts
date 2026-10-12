/**
 * Mock of `@/lib/ipc/generated/network`: the service registry with the Rust
 * defaults (LLM providers and model downloads on, everything else off) and a
 * short egress log of what a returning user's Mac sent today.
 */
import type * as Generated from '@/lib/ipc/generated/network';

import { latency } from './latency';
import { failure, ok } from './mock-database';

type ServiceId = Generated.ServiceId;

/** One registry entry (`deino_net::registry`). */
interface RegistryEntry {
  id: ServiceId;
  kind: Generated.ServiceKind;
  hosts: string[];
}

/** The registry's hosts and kind per service, in Rust's order. */
const REGISTRY: readonly RegistryEntry[] = [
  { id: 'llm_anthropic', kind: 'llm', hosts: ['api.anthropic.com'] },
  { id: 'llm_openai', kind: 'llm', hosts: ['api.openai.com'] },
  { id: 'llm_groq', kind: 'llm', hosts: ['api.groq.com'] },
  { id: 'llm_openrouter', kind: 'llm', hosts: ['openrouter.ai'] },
  { id: 'llm_custom', kind: 'llm', hosts: [] },
  { id: 'llm_ollama', kind: 'llm', hosts: ['localhost', '127.0.0.1', '[::1]'] },
  { id: 'hf_models', kind: 'models', hosts: ['huggingface.co', '*.huggingface.co', '*.hf.co'] },
  { id: 'google_calendar', kind: 'integration', hosts: ['www.googleapis.com'] },
  { id: 'google_drive', kind: 'integration', hosts: ['www.googleapis.com'] },
  { id: 'gmail', kind: 'integration', hosts: ['gmail.googleapis.com'] },
  { id: 'microsoft_graph', kind: 'integration', hosts: ['graph.microsoft.com'] },
  { id: 'slack', kind: 'integration', hosts: ['slack.com', '*.slack.com'] },
  { id: 'notion', kind: 'integration', hosts: ['api.notion.com'] },
  { id: 'linear', kind: 'integration', hosts: ['api.linear.app'] },
  { id: 'todoist', kind: 'integration', hosts: ['api.todoist.com', 'todoist.com'] },
  { id: 'asana', kind: 'integration', hosts: ['app.asana.com'] },
  { id: 'hubspot', kind: 'integration', hosts: ['api.hubapi.com'] },
  { id: 'attio', kind: 'integration', hosts: ['api.attio.com', 'app.attio.com'] },
  { id: 'salesforce', kind: 'integration', hosts: ['login.salesforce.com', '*.salesforce.com'] },
  {
    id: 'jira',
    kind: 'integration',
    hosts: ['api.atlassian.com', 'auth.atlassian.com', '*.atlassian.net'],
  },
  { id: 'monday', kind: 'integration', hosts: ['api.monday.com', 'auth.monday.com'] },
  { id: 'clickup', kind: 'integration', hosts: ['api.clickup.com'] },
  { id: 'trello', kind: 'integration', hosts: ['api.trello.com'] },
  { id: 'affinity', kind: 'integration', hosts: ['api.affinity.co'] },
  { id: 'zoho_crm', kind: 'integration', hosts: ['*.zoho.com', '*.zohoapis.com'] },
  { id: 'web_search', kind: 'web', hosts: ['api.search.brave.com', 'api.tavily.com'] },
  { id: 'webhooks', kind: 'web', hosts: [] },
  { id: 'favicons', kind: 'web', hosts: ['any public website'] },
  { id: 'granola', kind: 'integration', hosts: ['public-api.granola.ai'] },
  { id: 'circleback', kind: 'integration', hosts: ['circleback.ai'] },
  { id: 'fireflies', kind: 'integration', hosts: ['api.fireflies.ai'] },
  { id: 'fathom', kind: 'integration', hosts: ['api.fathom.ai'] },
];

/** The user's switches; services without one keep the Rust default. */
const switches = new Map<ServiceId, boolean>();

function status({ id, kind, hosts }: RegistryEntry): Generated.ServiceStatus {
  const enabled = switches.get(id) ?? (kind === 'llm' || kind === 'models');
  return { id, kind, hosts, enabled };
}

/** Milliseconds per minute. */
const MINUTE_MS = 60_000;

/** `minutes` ago, as Rust serializes a `DateTime<Utc>` (whole seconds). */
function ago(minutes: number): string {
  return new Date(Date.now() - minutes * MINUTE_MS).toISOString().replace(/\.\d+Z$/, 'Z');
}

/** Where a request went and how. */
type Request = Pick<Generated.EgressEntry, 'service' | 'host' | 'method' | 'status'>;

/** A sent request, `minutesAgo` minutes ago, with its bytes up and down. */
function sent(
  minutesAgo: number,
  request: Request,
  [bytesUp, bytesDown]: [number, number],
): Generated.EgressEntry {
  return { at: ago(minutesAgo), ...request, outcome: 'sent', bytesUp, bytesDown };
}

const CLAUDE: Request = {
  service: 'llm_anthropic',
  host: 'api.anthropic.com',
  method: 'POST',
  status: 200,
};

/** Newest first, like `egress_log`. */
const EGRESS_LOG: Generated.EgressEntry[] = [
  sent(2, CLAUDE, [48_213, 6_402]),
  sent(3, CLAUDE, [51_877, 7_118]),
  {
    ...sent(41, { service: 'slack', host: 'slack.com', method: 'POST', status: null }, [0, 0]),
    outcome: 'blocked',
  },
  sent(
    95,
    { service: 'hf_models', host: 'cas-bridge.xethub.hf.co', method: 'GET', status: 200 },
    [0, 731_357_568],
  ),
  sent(
    95,
    { service: 'hf_models', host: 'huggingface.co', method: 'GET', status: 302 },
    [0, 1_204],
  ),
];

/** Rust's cap on one page of the egress log (`MAX_EGRESS_PAGE`). */
const MAX_EGRESS_PAGE = 500;

export const commands = {
  async services() {
    await latency();
    return ok(REGISTRY.map(status));
  },
  async setServiceEnabled(service, enabled) {
    await latency();
    const entry = REGISTRY.find(({ id }) => id === service);
    if (entry === undefined) return failure({ code: 'invalid', message: 'unknown service' });
    switches.set(service, enabled);
    return ok(status(entry));
  },
  async egressLog(limit) {
    await latency();
    return ok(EGRESS_LOG.slice(0, Math.min(limit, MAX_EGRESS_PAGE)));
  },
} satisfies typeof Generated.commands;

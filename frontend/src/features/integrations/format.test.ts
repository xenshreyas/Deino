import { describe, expect, it } from 'vitest';

import { capabilitySummary, hasOAuth, lastUsedLabel, relativeTime, tokenMethod } from './format';
import type { ConnectMethod } from './types';

const NOW = new Date('2026-10-11T09:00:00Z');

describe('integration labels', () => {
  it('summarises capabilities in plain words', () => {
    expect(capabilitySummary(['post_message', 'create_task'])).toBe(
      'Posts messages · Creates tasks',
    );
  });

  it('says when a connection was last used', () => {
    expect(lastUsedLabel(null, NOW)).toBe('Not used yet');
    expect(lastUsedLabel('2026-10-11T08:59:40Z', NOW)).toBe('Last used just now');
    expect(relativeTime('2026-10-11T08:55:00Z', NOW)).toBe('5 minutes ago');
  });

  it('finds the token method and tells browser sign-ins apart', () => {
    const token: ConnectMethod = {
      method: 'token',
      label: 'API key',
      usernameLabel: null,
      helpUrl: 'https://x.invalid',
    };
    const oauth: ConnectMethod = {
      method: 'oauth',
      scopes: [],
      needsSecret: false,
      redirectUri: 'http://localhost:53682/callback',
      appSetupUrl: 'https://x.invalid',
    };

    expect(tokenMethod([oauth, token])).toBe(token);
    expect(tokenMethod([oauth])).toBeUndefined();
    expect(hasOAuth([oauth])).toBe(true);
    expect(hasOAuth([token])).toBe(false);
  });
});

import { describe, expect, it } from 'vitest';

import { actionFor, calendarPermission, missingRequired } from './permission-actions';
import type { PermissionStatus } from './types';

describe('the permission row action', () => {
  it('offers nothing once granted or where it is not needed', () => {
    expect(actionFor('granted')).toBe('none');
    expect(actionFor('unsupported')).toBe('none');
  });

  it('asks while unanswered or unknown', () => {
    expect(actionFor('not_determined')).toBe('request');
    expect(actionFor('unknown')).toBe('request');
  });

  it('sends a refused permission to System Settings', () => {
    expect(actionFor('denied')).toBe('settings');
  });
});

describe('calendar access as a permission', () => {
  it('maps every access level', () => {
    expect(calendarPermission('granted')).toBe('granted');
    expect(calendarPermission('not_determined')).toBe('not_determined');
    expect(calendarPermission('denied')).toBe('denied');
    expect(calendarPermission('restricted')).toBe('denied');
    expect(calendarPermission('unsupported')).toBe('unsupported');
  });

  it('treats write-only access as refused, since events cannot be read', () => {
    expect(calendarPermission('write_only')).toBe('denied');
  });
});

describe('the permissions recording needs', () => {
  const status = (overrides: Partial<PermissionStatus>): PermissionStatus => ({
    kind: 'microphone',
    state: 'granted',
    optional: false,
    ...overrides,
  });

  it('are missing while a required one needs action', () => {
    expect(missingRequired([status({ state: 'unknown', kind: 'system_audio' })])).toBe(true);
  });

  it('ignore optional permissions', () => {
    expect(
      missingRequired([
        status({}),
        status({ kind: 'accessibility', optional: true, state: 'denied' }),
      ]),
    ).toBe(false);
  });
});

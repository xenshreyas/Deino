import { describe, expect, it } from 'vitest';

import { builderCopy } from '../copy';
import { DEFAULT_FIELDS } from '../draft';
import { blockedBecause } from './step-choices';
import { shortDate } from './run-status';

describe('blockedBecause', () => {
  const script = { kind: 'run_shell', command: '', fields: DEFAULT_FIELDS } as const;

  it('blocks a script step until shell scripts are allowed', () => {
    expect(blockedBecause('run_shell', script, false)).toBe(builderCopy.needs.run_shell);
    expect(blockedBecause('run_shell', script, true)).toBeNull();
  });

  it('names what a step needs when there is nothing for it to point at', () => {
    expect(blockedBecause('add_tag', null, true)).toBe(builderCopy.needs.add_tag);
    expect(blockedBecause('generate_insight', null, true)).toBe(builderCopy.needs.generate_insight);
    expect(blockedBecause('add_to_folder', null, true)).toBe(builderCopy.needs.add_to_folder);
  });

  it('lets a step that can start through', () => {
    expect(blockedBecause('notify', { kind: 'notify', message: '' }, false)).toBeNull();
  });
});

describe('shortDate', () => {
  it('leaves out the year only for this year', () => {
    const thisYear = new Date().getFullYear();
    expect(shortDate(`${String(thisYear)}-03-04T12:00:00`)).toBe('Mar 4');
    expect(shortDate('2019-03-04T12:00:00')).toBe('Mar 4, 2019');
  });
});

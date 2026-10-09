import { describe, expect, it } from 'vitest';

import { toForm, toInput } from './form';

describe('the profile form', () => {
  it('edits lists as text and sends them back as lists', () => {
    const form = toForm({
      name: 'Shreyas',
      emails: ['shreyas@deino.dev', 'shreyas.rao@gmail.com'],
      role: 'CTO',
      company: 'Deino',
      focusAreas: ['Hiring', 'Reliability'],
      writingStyle: '',
      context: '',
    });

    expect(form.emails).toBe('shreyas@deino.dev\nshreyas.rao@gmail.com');
    expect(form.focusAreas).toBe('Hiring, Reliability');
    expect(toInput({ ...form, emails: 'a@deino.dev, b@deino.dev\n' }).emails).toEqual([
      'a@deino.dev',
      'b@deino.dev',
      '',
    ]);
    expect(toInput(form).focusAreas).toEqual(['Hiring', 'Reliability']);
  });
});

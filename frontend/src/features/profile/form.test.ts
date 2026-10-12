import { describe, expect, it } from 'vitest';

import { isDirty, toForm, toInput } from './form';

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

  it('is dirty once any field differs from the saved form', () => {
    const saved = toForm({
      name: 'Shreyas',
      emails: [],
      role: '',
      company: '',
      focusAreas: [],
      writingStyle: '',
      context: '',
    });

    expect(isDirty({ ...saved }, saved)).toBe(false);
    expect(isDirty({ ...saved, company: 'Deino' }, saved)).toBe(true);
    expect(isDirty({ ...saved, context: 'FSM' }, saved)).toBe(true);
  });
});

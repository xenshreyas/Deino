import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';

import { ThemePicker } from './theme-picker';

describe('ThemePicker', () => {
  test('marks the current choice and reports a new one', () => {
    const onChange = vi.fn();
    render(<ThemePicker value="system" onChange={onChange} />);

    expect(screen.getByRole('radio', { name: 'System' }).getAttribute('aria-checked')).toBe('true');
    expect(screen.getByRole('radio', { name: 'Dark' }).getAttribute('aria-checked')).toBe('false');

    fireEvent.click(screen.getByRole('radio', { name: 'Dark' }));
    expect(onChange).toHaveBeenCalledWith('dark');
  });
});

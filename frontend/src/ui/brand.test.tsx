import { render, screen } from '@testing-library/react';
import { describe, expect, test } from 'vitest';

import { BrandMark, Wordmark } from './brand';

describe('brand', () => {
  test('the wordmark is an image named Deino, painted with the foreground token', () => {
    render(<Wordmark size={20} />);
    const wordmark = screen.getByRole('img', { name: 'Deino' });
    expect(wordmark.className).toContain('bg-fg');
    expect(wordmark.style.height).toBe('20px');
    expect(wordmark.style.maskImage).toContain('wordmark');
  });

  test('the mark is decorative by default, painted with the brand token', () => {
    const { container } = render(<BrandMark />);
    const mark = container.querySelector('span');
    expect(screen.queryByRole('img')).toBeNull();
    expect(mark?.getAttribute('aria-hidden')).toBe('true');
    expect(mark?.className).toContain('bg-brand');
    expect(mark?.style.height).toBe('16px');
    expect(mark?.style.maskImage).toContain('mark');
  });

  test('a labelled mark is an image with that name', () => {
    render(<BrandMark label="Deino" size={40} />);
    expect(screen.getByRole('img', { name: 'Deino' }).style.height).toBe('40px');
  });
});

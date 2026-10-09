import { render, screen } from '@testing-library/react';
import { describe, expect, test } from 'vitest';

import { EmptyStateSummary } from './EmptyStateSummary';

describe('empty summary state', () => {
  test('renders a persistent retryable error', () => {
    render(<EmptyStateSummary error="Summary request failed" hasModel onGenerate={() => {}} />);

    expect(screen.getByRole('alert').textContent).toContain('Summary request failed');
    expect(screen.getByText('Retry summary')).toBeDefined();
  });
});

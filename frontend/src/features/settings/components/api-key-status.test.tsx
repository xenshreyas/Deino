import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { settingsCopy } from '../copy';
import { ApiKeyStatus } from './api-key-status';

describe('ApiKeyStatus', () => {
  it('asks the user to enter a key again when macOS refuses access to it', () => {
    render(<ApiKeyStatus status="locked" />);

    expect(screen.getByRole('alert').textContent).toBe(settingsCopy.keyLocked);
  });

  it('says a saved key exists without showing anything of it', () => {
    render(<ApiKeyStatus status="stored" />);

    expect(screen.getByText(settingsCopy.keyStored)).toBeTruthy();
  });

  it('shows nothing when no key is saved', () => {
    const { container } = render(<ApiKeyStatus status="missing" />);

    expect(container.innerHTML).toBe('');
  });
});

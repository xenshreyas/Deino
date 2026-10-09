import { settingsCopy } from '../copy';
import type { SecretStatus } from '../types';

interface ApiKeyStatusProps {
  /** The key's status; nothing is shown while it loads or when none is saved. */
  status: SecretStatus | undefined;
}

/**
 * One line under an API key field: whether a key is already saved, or that
 * macOS refused access to it and the user should enter it again (ad-hoc
 * builds lose Keychain access when their signature changes).
 */
export function ApiKeyStatus({ status }: ApiKeyStatusProps) {
  if (status === 'stored') {
    return <p className="mt-1 text-xs text-fg-muted">{settingsCopy.keyStored}</p>;
  }
  if (status === 'locked') {
    return (
      <p role="alert" className="mt-1 text-xs text-danger">
        {settingsCopy.keyLocked}
      </p>
    );
  }
  return null;
}

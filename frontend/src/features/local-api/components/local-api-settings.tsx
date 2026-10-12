import { KeysSection } from './keys-section';
import { ServerSection } from './server-section';

/**
 * Settings → Developer's local REST API (DEV-03): the server's switch and
 * port, and the keys that may call it.
 */
export function LocalApiSettings() {
  return (
    <div className="flex flex-col gap-8">
      <ServerSection />
      <KeysSection />
    </div>
  );
}

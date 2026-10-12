/**
 * Settings → Automations → "Allow shell scripts" (`automations.shellScripts`,
 * off by default). Rust checks it before every script runs.
 */
import { usePatchSettings, useSettings } from '@/features/settings';

/** Whether scripts may run (`undefined` while loading) and its setter. */
export function useShellScripts(): {
  allowed: boolean | undefined;
  setAllowed: (allowed: boolean) => void;
} {
  const { data } = useSettings();
  const { mutate } = usePatchSettings();
  return {
    allowed: data?.automations.shellScripts,
    setAllowed: (allowed) => {
      mutate({ automations: { shellScripts: allowed } });
    },
  };
}

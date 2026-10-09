import { useEffect, type EffectCallback } from 'react';

/**
 * Runs `effect` once after the first render and its cleanup on unmount.
 *
 * The only sanctioned way to sync with an external system on mount (PARITY_PLAN
 * §3 rule 4, after anarlog's `useMountEffect`, MIT). Anything that should
 * restart when a value changes belongs in a component remounted with `key`.
 */
export function useMountEffect(effect: EffectCallback): void {
  // Mount-only by design: the empty dependency list is the point of this hook.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(effect, []);
}

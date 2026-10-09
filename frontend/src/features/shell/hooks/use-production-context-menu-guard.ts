import { useMountEffect } from '@/hooks/useMountEffect';

/** Stops the webview's native context menu ("Reload", "Inspect"). */
function preventContextMenu(event: MouseEvent): void {
  event.preventDefault();
}

/**
 * Hides the webview's native right-click menu in production builds.
 *
 * Development builds keep it for "Inspect Element".
 */
export function useProductionContextMenuGuard(): void {
  useMountEffect(() => {
    if (!import.meta.env.PROD) return undefined;
    document.addEventListener('contextmenu', preventContextMenu);
    return () => document.removeEventListener('contextmenu', preventContextMenu);
  });
}

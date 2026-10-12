/**
 * Entry of the record panel window (`record-panel.html`, CAP-10): a
 * transparent, always-on-top, non-activating panel that shows only the
 * pill. No router and no shell, so it loads fast and stays small; its
 * capability (`capabilities/record-panel.json`) grants only the recording
 * plugin, window dragging, the microphone menu and the settings (for
 * PRV-01's once-shown pause tip).
 */
import '@fontsource-variable/inter';
import './styles/globals.css';

import { createRoot } from 'react-dom/client';

import { applyTheme, bootTheme, parseThemePreference } from '@/features/appearance';
import { createQueryClient, IpcQueryProvider } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

// `pnpm dev:mock`: the panel in a plain browser, on a demo scene
// (`?demo=collapsed|expanded|warning|paused|ending|split&theme=dark`).
if (import.meta.env.MODE === 'mock') {
  await import('@/features/meetings');
  const { installMockIpc } = await import('@/dev/mock-ipc/install');
  installMockIpc();
  const { isPanelScene, startPanelDemo } = await import('@/dev/mock-ipc/mock-panel-demo');
  const params = new URLSearchParams(window.location.search);
  const scene = params.get('demo');
  if (params.has('theme')) applyTheme(parseThemePreference(params.get('theme')));
  // After the first render, so the page's listeners hear the scene.
  window.setTimeout(() => {
    void startPanelDemo(isPanelScene(scene) ? scene : 'collapsed');
  }, 300);
}

const rootElement = document.getElementById('root');
if (!rootElement) {
  throw new Error('record-panel.html is missing the #root element');
}

// Loaded after the mock IPC is in place: the feature's bindings resolve to
// the mocks in `pnpm dev:mock`, whose modules import each other.
const { RecordPanel } = await import('@/features/recording');

bootTheme();
// The window is transparent; only the pill is drawn.
document.body.classList.add('bg-transparent');

createRoot(rootElement).render(
  <IpcQueryProvider client={createQueryClient()}>
    <TooltipProvider delayDuration={500}>
      <RecordPanel />
    </TooltipProvider>
  </IpcQueryProvider>,
);

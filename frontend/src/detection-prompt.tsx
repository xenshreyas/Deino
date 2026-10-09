/**
 * Entry of the meeting prompt window (`detection-prompt.html`, CAP-06): a
 * transparent, always-on-top panel that shows only the prompt card. It has
 * no router and no shell, so it loads fast and stays small; its capability
 * (`capabilities/detection-prompt.json`) grants only the detection plugin.
 */
import '@fontsource-variable/inter';
import './styles/globals.css';

import { createRoot } from 'react-dom/client';

import { bootTheme } from '@/features/appearance';
import { DetectionPrompt } from '@/features/detection';
import { createQueryClient, IpcQueryProvider } from '@/lib/ipc';

// `pnpm dev:mock`: the prompt in a plain browser on a fixture detection.
if (import.meta.env.MODE === 'mock') {
  // The mock database and the meetings feature import each other; the app's
  // entry loads the feature first through its routes, so this page does too.
  await import('@/features/meetings');
  const { installMockIpc } = await import('@/dev/mock-ipc/install');
  installMockIpc();
}

const rootElement = document.getElementById('root');
if (!rootElement) {
  throw new Error('detection-prompt.html is missing the #root element');
}

bootTheme();
// The window is transparent; only the card is drawn. The inset leaves room
// for the card's shadow inside the window's bounds.
document.body.classList.add('bg-transparent', 'p-2');

createRoot(rootElement).render(
  <IpcQueryProvider client={createQueryClient()}>
    <DetectionPrompt />
  </IpcQueryProvider>,
);

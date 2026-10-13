/**
 * Entry of the meeting reminder window (`meeting-reminder.html`, ORG-02): a
 * transparent, always-on-top panel that shows only the "starting in a
 * minute" card. It has no router and no shell, so it loads fast and stays
 * small; its capability (`capabilities/meeting-reminder.json`) grants only
 * the calendar plugin.
 */
import '@fontsource-variable/inter';
import './styles/globals.css';

import { createRoot } from 'react-dom/client';

import { bootTheme } from '@/features/appearance';
import { MeetingReminder } from '@/features/calendar';
import { createQueryClient, IpcQueryProvider } from '@/lib/ipc';

// `pnpm dev:mock`: the card in a plain browser on the next fixture event.
if (import.meta.env.MODE === 'mock') {
  // The mock database and the meetings feature import each other; the app's
  // entry loads the feature first through its routes, so this page does too.
  await import('@/features/meetings');
  const { installMockIpc } = await import('@/dev/mock-ipc/install');
  installMockIpc();
}

const rootElement = document.getElementById('root');
if (!rootElement) {
  throw new Error('meeting-reminder.html is missing the #root element');
}

bootTheme();
// The window is transparent; only the card is drawn. The window reaches the
// screen's right edge (so the card can slide in from off-screen); the inset
// leaves room for the card's shadow and keeps it 16px from that edge.
document.body.classList.add('bg-transparent', 'p-2', 'pr-4');

createRoot(rootElement).render(
  <IpcQueryProvider client={createQueryClient()}>
    <MeetingReminder />
  </IpcQueryProvider>,
);

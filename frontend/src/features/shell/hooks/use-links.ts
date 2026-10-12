/**
 * `deino://` links and the system-wide hotkeys (DEV-05) in the shell: the
 * link host, with "Ask" opening the right panel (whose first section is
 * Ask) when a meeting is open. Elsewhere Rust has already brought Deino
 * forward, which is all Ask does there.
 */
import { useRouter } from '@tanstack/react-router';

import { useLinkHost } from '@/features/links';

import { useShellLayout } from '../layout-store';

/** The meeting page's route. */
const MEETING_ROUTE = '/meetings/$meetingId';

/** Follows links and hotkeys for the lifetime of the main layout. */
export function useLinks(): void {
  const router = useRouter();
  useLinkHost({
    openAsk: () => {
      const onMeeting = router.state.matches.some((match) => match.routeId === MEETING_ROUTE);
      if (onMeeting) useShellLayout.setState({ rightPanelOpen: true });
    },
  });
}

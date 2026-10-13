/**
 * The logo on the reminder's Join button: the call's service (Google Meet,
 * Zoom, Teams), or the Deino mark for any other service or no call at all.
 * The files and their sources are in `../assets/README.md`.
 */
import { BrandMark } from '@/ui/brand';

import googleMeet from '../assets/google-meet.svg';
import teams from '../assets/teams.svg';
import zoom from '../assets/zoom.svg';
import type { CalendarEvent } from '../types';

/** The services with a logo of their own. */
const LOGOS: Partial<Record<NonNullable<CalendarEvent['platform']>, string>> = {
  google_meet: googleMeet,
  zoom,
  teams,
};

/** The service's logo at 18px, decorative (the button says what it does). */
export function ServiceLogo({ platform }: { platform: CalendarEvent['platform'] }) {
  const logo = platform === null ? undefined : LOGOS[platform];
  if (logo === undefined) return <BrandMark size={16} />;
  return <img src={logo} alt="" aria-hidden className="size-[18px] shrink-0" />;
}

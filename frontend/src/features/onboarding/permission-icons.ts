/** Each permission's icon, shared by setup and the in-context asks. */
import { AudioLines, Bell, type Icon, Mic, Monitor, Users } from '@/ui/icons';

import type { PermissionKind } from './types';

export const PERMISSION_ICONS: Record<PermissionKind, Icon> = {
  microphone: Mic,
  system_audio: AudioLines,
  notifications: Bell,
  accessibility: Users,
  screen_recording: Monitor,
};

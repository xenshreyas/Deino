/**
 * The toast after a speaker is named, by rename or by "Yes". The new name
 * shows in the transcript, so only what it cannot show is said (UX-32):
 * that the voice was saved for next time.
 */
import { toast } from 'sonner';

import { speakersCopy } from './copy';
import type { Renamed } from './types';

/** "Deino will recognise Maya Chen’s voice next time", when her voice was saved. */
export function announceRename(renamed: Renamed): void {
  if (renamed.enrolled) toast.success(speakersCopy.remembered(renamed.person.name));
}

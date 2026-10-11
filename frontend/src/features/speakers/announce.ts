/** The toast after a speaker is named, by rename or by "Yes". */
import { toast } from 'sonner';

import { speakersCopy } from './copy';
import type { Renamed } from './types';

/** "Named Maya Chen", and that her voice will be recognised when it was saved. */
export function announceRename(renamed: Renamed): void {
  const { name } = renamed.person;
  const description = renamed.enrolled ? speakersCopy.remembered(name) : undefined;
  toast.success(speakersCopy.renamed(name), { description });
}

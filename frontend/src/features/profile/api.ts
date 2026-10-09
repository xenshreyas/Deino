/**
 * Typed calls to the `profile` plugin. The profile itself is read with the
 * settings (`useSettings().data.profile`).
 */
import { commands, type ProfileInput, type ProfileSettings } from '@/lib/ipc/generated/profile';
import { unwrap } from '@/lib/ipc/result';

export const profileApi = {
  /** Normalises and saves the profile; rejects with `invalid` naming the field. */
  update: (input: ProfileInput): Promise<ProfileSettings> => unwrap(commands.update(input)),
  /** The context every AI prompt starts with, or `null` for an empty profile. */
  context: (): Promise<string | null> => unwrap(commands.context()),
};

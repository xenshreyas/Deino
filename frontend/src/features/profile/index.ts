/**
 * The profile feature (NOT-05): Settings → Profile, saved through the
 * `profile` plugin, which also keeps the "Me" person in step; and the
 * preview of the context every AI prompt starts with.
 */
export { profileApi } from './api';
export { ProfilePage } from './components/profile-page';
export { profileContextQuery, profileKeys, useProfileContext, useSaveProfile } from './queries';

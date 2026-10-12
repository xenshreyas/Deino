/**
 * The profile feature (NOT-05): Settings → Profile, saved through the
 * `profile` plugin, which also keeps the "Me" person in step; and the
 * preview of the context every AI prompt starts with.
 */
export { profileApi } from './api';
export { AboutYouFields } from './components/profile-fields';
export { ProfilePage } from './components/profile-page';
export { type ProfileFormState, useProfileForm } from './hooks/use-profile-form';
export { profileContextQuery, profileKeys, useProfileContext, useSaveProfile } from './queries';

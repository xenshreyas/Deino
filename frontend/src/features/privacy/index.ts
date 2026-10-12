/**
 * Encryption at rest and the app lock (PRV-02). Rust converts the database
 * at startup and refuses library reads while locked; this feature shows the
 * startup and lock screens and the Settings page.
 *
 * | Export | Where |
 * |---|---|
 * | {@link LibraryGate} | the root route, around the app shell: progress and key-problem screens |
 * | {@link LockGate} | the app shell, around the main layout: the lock screen and activity pings |
 * | {@link PrivacySettings} | Settings → Privacy & security |
 */
export { privacyApi } from './api';
export { LibraryGate } from './components/library-gate';
export { LockGate } from './components/lock-gate';
export { PrivacySettings } from './components/privacy-settings';
export { privacyCopy } from './copy';
export { privacyKeys, useLockActions, useLockState } from './queries';
export type { LibraryStatus, LockState } from './types';

import type { Direction, LibraryProblem } from './types';

/** Every visible string of the privacy feature (PRV-02). */
export const privacyCopy = {
  lock: {
    title: 'Deino is locked',
    description: 'Your meetings and notes are hidden until you unlock.',
    unlock: 'Unlock with Touch ID',
    unlocking: 'Waiting for Touch ID…',
    recording: 'Recording continues while Deino is locked.',
    failed: 'Could not unlock Deino',
  },

  library: {
    converting: (direction: Direction) =>
      direction === 'encrypt' ? 'Encrypting your library…' : 'Decrypting your library…',
    convertingDetail:
      'This takes a moment for a large library. Your meetings stay as they are until the new copy is checked.',
    unavailableTitle: 'Deino cannot open your library',
    problems: {
      key_missing:
        'Your library is encrypted, but its key is not in your Keychain. Restore the Keychain from a backup, or turn encryption back on in the settings file. Nothing was deleted.',
      key_denied:
        'macOS did not let Deino read the library key. Quit Deino, open it again and choose Allow (or Always Allow) when the Keychain asks.',
      wrong_key:
        'The key in your Keychain does not open this library. Nothing was deleted; restore the matching Keychain item from a backup.',
      open_failed: 'Something went wrong while opening the library. The details are in the log.',
    } satisfies Record<LibraryProblem, string>,
  },

  settings: {
    encryptionTitle: 'Encryption',
    encryptionDescription: 'Keep transcripts, notes and people encrypted on disk.',
    encryptTitle: 'Encrypt the library',
    encryptedNow: 'On. The library is encrypted with a key kept in your Keychain.',
    plaintextNow: 'Off. The library is a plain SQLite file, protected by FileVault if it is on.',
    restartToEncrypt: 'Restart Deino to encrypt the library.',
    restartToDecrypt: 'Restart Deino to decrypt the library.',
    failed: (direction: Direction) =>
      direction === 'encrypt'
        ? 'The last start could not encrypt the library; it is unchanged. Deino tries again at the next start.'
        : 'The last start could not decrypt the library; it is still encrypted. Deino tries again at the next start.',
    confirmEncryptTitle: 'Encrypt the library?',
    confirmEncrypt: [
      'The next time Deino starts, it encrypts its database with a new random key and saves the key in your Keychain first.',
      'The original file stays until the encrypted copy is checked, so nothing is lost if the Mac shuts down halfway. A large library takes a moment.',
      'Recordings are not encrypted by Deino. Keep them on an encrypted volume (see below).',
    ],
    confirmDecryptTitle: 'Decrypt the library?',
    confirmDecrypt: [
      'The next time Deino starts, it writes a plain copy of the database, checks it, and only then forgets the key in your Keychain.',
    ],
    confirmCancel: 'Cancel',
    confirmEncryptAction: 'Encrypt at next start',
    confirmDecryptAction: 'Decrypt at next start',

    lockTitle: 'App lock',
    lockDescription: 'Hide your meetings behind Touch ID or your Mac password.',
    lockSwitch: 'Lock Deino',
    lockOn: 'Recording keeps going while Deino is locked.',
    lockOff: 'Deino stays open while your Mac is unlocked.',
    idleTitle: 'Lock after',
    idleOption: (minutes: number) =>
      minutes < 60 ? `${String(minutes)} minutes` : `${String(minutes / 60)} hour`,
    idleDescription: 'Without using Deino.',
    wakeTitle: 'Lock when the Mac wakes',
    wakeDescription: 'Also when the screen locks.',
    lockNow: 'Lock now',

    recordingsTitle: 'Recordings',
    recordingsDescription: 'Audio files are not encrypted by Deino.',
    recordingsBody: [
      'FileVault already encrypts every recording while the Mac is off or logged out.',
      'For a second lock, create an encrypted APFS volume in Disk Utility and move the recordings folder onto it while Deino is quit. docs/setup/encrypted-recordings.md has the steps.',
    ],

    saveFailed: 'Could not save the privacy setting',
  },
} as const;

# Keep recordings on an encrypted APFS volume

Deino can encrypt its database (Settings → Privacy & security, PRV-02,
ADR 0041). Audio files are not encrypted by Deino: they stay plain `.m4a`
and `.mp4` files so playback, export and Finder previews keep working. If
you want them behind their own password, move the recordings folder to an
encrypted APFS volume.

First, check FileVault (System Settings → Privacy & Security → FileVault).
With FileVault on, everything on the disk, recordings included, is already
encrypted at rest while the Mac is off or logged out. An encrypted volume
adds a second lock: the recordings stay unreadable until you unlock that
volume, even while you are logged in.

## 1. Create the volume

1. Open **Disk Utility**, select your startup container (usually
   "Macintosh HD"'s container) and click **+** (Add APFS Volume).
2. Name it `Deino Recordings`, format **APFS (Encrypted)**, choose a
   password, and click **Add**. APFS volumes share the container's free
   space, so no size is needed.
3. Optional: when macOS asks, save the password in your Keychain so the
   volume unlocks when you log in. Without it, you unlock the volume by hand
   in Finder before recording.

## 2. Move the recordings folder

Quit Deino first (Deino → Quit, and make sure no recording is running).
Then, in Terminal:

```bash
data="$HOME/Library/Application Support/com.xenshreyas.deino"
target="/Volumes/Deino Recordings/recordings"
mkdir -p "$target"
rsync -a "$data/recordings/" "$target/"     # copy everything, keep dates
diff -rq "$data/recordings" "$target" && \
  mv "$data/recordings" "$data/recordings.old" && \
  ln -s "$target" "$data/recordings"
```

Open Deino, play back an older meeting and make a short test recording.
When both work, delete `recordings.old`.

## Things to know

- **Unlock before you record.** If the volume is locked, the link points at
  nothing and Deino cannot start a recording. Unlock the volume in Finder
  (or save its password in the Keychain) first.
- **Undo:** quit Deino, delete the `recordings` link, then move the folder
  back from the volume (`mv "/Volumes/Deino Recordings/recordings" "$data/"`).
- **Backups:** Time Machine backs up the encrypted volume like any other;
  encrypt the backup disk too.
- The database (transcripts, notes, people) is separate: encrypt it in
  Settings → Privacy & security.

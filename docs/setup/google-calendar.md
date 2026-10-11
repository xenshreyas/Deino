# Google Calendar connector

Deino reads Google calendars through **macOS Calendar** by default: add the
Google account in System Settings → Internet Accounts, turn on Calendars, and
it appears in Settings → Calendar with no further setup.

The direct connector is for a Google account you do not want on the Mac. It
signs in from Deino with your own OAuth client, reads with the
`calendar.readonly` scope, and sends events from Google straight to this
Mac (ADR 0027). Deino has no OAuth client of its own; you create one once.

## 1. Create the Google Cloud project

1. Open <https://console.cloud.google.com/> and create a project (for
   example "Deino").
2. **APIs & Services → Library**: enable the **Google Calendar API**.

## 2. Configure the consent screen

1. **APIs & Services → OAuth consent screen** (Google Auth Platform →
   Branding / Audience / Data access).
2. User type **External**; app name "Deino"; your address as support and
   developer contact.
3. **Data access → Add or remove scopes**: add
   `https://www.googleapis.com/auth/calendar.readonly`.
4. **Audience**: leave the publishing status at **Testing** and add your
   Google address under **Test users**.

> In Testing, Google expires refresh tokens after **7 days**: Settings →
> Calendar then shows the sign-in error and you click **Connect** again.
> Publishing the app ("In production") removes the limit; Google then shows
> an "unverified app" warning on sign-in, which is expected for a personal
> client.

## 3. Create the OAuth client

1. **APIs & Services → Credentials → Create credentials → OAuth client ID**.
2. Application type **Desktop app**, name "Deino".
3. Copy the **Client ID** (`….apps.googleusercontent.com`) and the
   **Client secret** (`GOCSPX-…`).

A desktop client needs no redirect URI: Google accepts any loopback port,
and Deino listens on `http://127.0.0.1:<free port>/callback` only while you
sign in. Google calls the desktop secret "not treated as a secret"; Deino
still keeps it in the Keychain.

## 4. Connect in Deino

1. Settings → Calendar → **Direct connections → Google Calendar**.
2. Paste the client ID (saved in `settings.json` as
   `calendar.google.clientId`) and the client secret (saved in the Keychain
   as `google_oauth_client_secret`).
3. Click **Connect**. Your browser opens Google's sign-in page; choose the
   account and allow read-only calendar access. The browser then shows
   "Signed in to Google" and Deino syncs.

Connecting switches the **Google Calendar** service on in Settings →
Network (requests go to `www.googleapis.com` and `oauth2.googleapis.com`
only, and appear in the egress log). The refresh token is kept in the
Keychain (`google_calendar_refresh_token`).

**Disconnect** forgets the refresh token, removes that account's synced
calendars and events (meetings stay), and switches the service off. To
revoke Deino entirely, also remove it at
<https://myaccount.google.com/permissions>.

## Troubleshooting

| Symptom | Fix |
|---|---|
| Connect stays disabled | Both the client ID and the client secret must be saved. |
| `access_denied` / "app not verified" blocks sign-in | Add your address as a test user (step 2.4). |
| "redirect_uri_mismatch" | The client is not a Desktop app; create a Desktop client (step 3). |
| Sync shows a sign-in error after a week | Testing-mode token expiry; click Connect again, or publish the app. |
| Nothing syncs, no error | Settings → Network: offline mode is on, or the Google Calendar service was switched off. |

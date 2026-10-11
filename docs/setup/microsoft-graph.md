# Microsoft (Outlook) calendar connector

Deino reads Outlook and Exchange calendars through **macOS Calendar** by
default: add the account in System Settings → Internet Accounts (Microsoft
Exchange), turn on Calendars, and it appears in Settings → Calendar with no
further setup.

The direct connector is for an account you do not want on the Mac, or one
whose organisation blocks Exchange sync on personal Macs. It signs in from
Deino with your own Microsoft Entra app registration, reads through Microsoft
Graph with `Calendars.Read`, and sends events from Microsoft straight to
this Mac (ADR 0027). Deino has no app registration of its own; you create
one once.

## 1. Register the app

1. Open <https://entra.microsoft.com/> → **Identity → Applications → App
   registrations → New registration** (or the same page in the Azure portal).
2. Name "Deino".
3. **Supported account types:** "Accounts in any organizational directory and
   personal Microsoft accounts" (Deino signs in through the `common`
   endpoint, so work, school and personal accounts all work). A single-tenant
   registration also works for accounts of that tenant.
4. **Redirect URI:** platform **Public client/native (mobile & desktop)**,
   URI `http://localhost`. Entra matches any port on `localhost`, and Deino
   listens on a free port only while you sign in.
5. Register, then copy the **Application (client) ID** from Overview.

No client secret: Deino is a public client and proves itself with PKCE.

## 2. API permissions

**API permissions → Add a permission → Microsoft Graph → Delegated
permissions**: `Calendars.Read`, `User.Read`, `offline_access`. These need
no admin consent for your own account. If your organisation requires admin
consent for third-party apps, an administrator must grant it (or use macOS
Calendar instead).

## 3. Connect in Deino

1. Settings → Calendar → **Direct connections → Microsoft Outlook**.
2. Paste the client ID (saved in `settings.json` as
   `calendar.microsoft.clientId`).
3. Click **Connect**. Your browser opens Microsoft's sign-in page; choose the
   account and accept the read-only permissions. The browser then shows
   "Signed in to Microsoft" and Deino syncs.

Connecting switches the **Microsoft Graph** service on in Settings → Network
(requests go to `graph.microsoft.com` and `login.microsoftonline.com` only,
and appear in the egress log). The refresh token is kept in the Keychain
(`microsoft_calendar_refresh_token`).

**Disconnect** forgets the refresh token, removes that account's synced
calendars and events (meetings stay), and switches the service off. To
revoke Deino entirely, remove it at <https://myapps.microsoft.com/> (work or
school) or <https://account.live.com/consent/Manage> (personal).

## Troubleshooting

| Symptom | Fix |
|---|---|
| Connect stays disabled | Save the client ID first. |
| `AADSTS50011` (redirect URI mismatch) | Add `http://localhost` under the **Mobile and desktop** platform, not Web or SPA. |
| `AADSTS7000218` (client secret required) | Under Authentication, set "Allow public client flows" to Yes, or check the platform is Mobile and desktop. |
| "Need admin approval" | Your organisation restricts consent; ask an administrator, or use macOS Calendar. |
| Personal account rejected | The registration supports organisational accounts only; change Supported account types (step 1.3). |
| Nothing syncs, no error | Settings → Network: offline mode is on, or the Microsoft Graph service was switched off. |

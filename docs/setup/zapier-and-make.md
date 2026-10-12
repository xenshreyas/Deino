# Send meetings to Zapier or Make (webhooks)

Deino can post each meeting as signed JSON to a URL you choose. Zapier and
Make both give you such a URL, so a finished meeting can create a CRM note,
a task, a row in a sheet, or anything else they connect to.

Deino sends from your Mac, directly to the URL. There is no Deino server in
between. Webhooks are sent only while Deino is running, offline mode stops
them, and Settings → Privacy → Network lists every request.

## 1. Get a URL

**Zapier**

1. Create a Zap. For the trigger choose **Webhooks by Zapier**, then
   **Catch Hook**.
2. Leave "Pick off a Child Key" empty and click **Continue**.
3. Copy the URL Zapier shows (`https://hooks.zapier.com/hooks/catch/…`).

**Make**

1. Create a scenario. Add the **Webhooks** module, then **Custom webhook**.
2. Click **Add**, name it (for example "Deino"), and click **Save**.
3. Copy the URL Make shows (`https://hook.<region>.make.com/…`).

## 2. Add the webhook in Deino

1. Open **Settings → Developer → Webhooks** and click **Add webhook**.
2. Paste the URL. It must start with `https://`. Only `http://localhost`
   is allowed without TLS, for testing your own server.
3. Under **Send when**, pick the events:

   | Event | Sent when |
   |---|---|
   | `meeting.ready` | The meeting has finished processing |
   | `notes.generated` | AI notes are written for the first time |
   | `notes.edited` | AI notes are edited or written again (one request after a 30 s pause) |
   | `action_item.created` | An action item is added, by you or by extraction |
   | `action_item.updated` | An action item changes, is completed or reopened |
   | `insight.generated` | Custom insights are answered |

4. Under **Payload includes**, pick what to send: notes, action items,
   transcript, insights, attendees, the `deino://` link, and the audio path.
   The meeting's ID, title, start time, duration, tags and folders are
   always included. With **Settings → Privacy → Hide sensitive information**
   on, passwords, keys and card numbers are `[redacted]` in the payload,
   just as they are in exports.
5. Click **Add**. Deino shows the **signing secret** (`whsec_…`) once.
   Copy it now if you will verify signatures. If you lose it, choose ⋯ →
   **New secret**.

## 3. Send a test

Click **Send test**. Deino sends your newest finished meeting, with `type`
set to `test`. Zapier and Make use this request to learn the fields:

- **Zapier:** click **Test trigger**.
- **Make:** click **Run once** before you click **Send test**.

Then map the fields in your next steps. **Deliveries** shows every request,
its HTTP status and any retries.

## The request

Deino sends a `POST` with `content-type: application/json`:

```json
{
  "type": "meeting.ready",
  "timestamp": "2026-10-12T16:31:02Z",
  "data": {
    "id": "meeting-…",
    "name": "Acme sync",
    "createdAt": "2026-10-12T09:00:00-07:00",
    "duration": 1800,
    "tags": ["Sales"],
    "folders": ["Clients"],
    "url": "deino://meetings/meeting-…",
    "attendees": [{ "name": "Maya Chen", "email": "maya@acme.com", "isMe": false }],
    "notes": "## Summary\n…",
    "actionItems": [{ "title": "Send the quote", "description": "", "assignee": "Maya Chen", "due": null, "done": false }],
    "transcript": [{ "speaker": "Maya Chen", "isMe": false, "start": 0, "end": 900, "text": "…" }],
    "insights": []
  }
}
```

`action_item.*` events also carry `data.actionItem` (`id`, `title`,
`description`, `status`, `dueAt`). An automation's webhook step (Automations
→ step **Webhook**) sends the automation payload (`format:
"deino.automation"`) to one of these endpoints instead.

## Verifying the signature (optional)

Each request carries two signatures made with the secret. Check either one.

- **Standard Webhooks** (`webhook-id`, `webhook-timestamp`,
  `webhook-signature: v1,<base64>`): the HMAC-SHA256 of
  `<webhook-id>.<webhook-timestamp>.<raw body>`, keyed with the
  base64-decoded part of the secret after `whsec_`. Any
  [Standard Webhooks library](https://www.standardwebhooks.com/) verifies it
  with the secret as shown. Reject timestamps more than 5 minutes old.
- **`x-signature`** (hex): the HMAC-SHA256 of the raw body, keyed with the
  whole secret as shown (`whsec_…`, as text). In Make, use the
  `sha256(body; "hex"; secret)` function on the raw body and compare it with
  the header. In Zapier, use a Code step.

`webhook-id` is the same on every retry of one event, so a receiver can drop
duplicates.

## Retries and turning off

- Each attempt may take 15 seconds.
- A timeout, a network error, `408`, `429` or `5xx` is retried after 30 s,
  then 1, 2, 4… minutes, at most every 2 hours, for 24 hours. Any other
  `3xx` or `4xx` is not retried. Retries continue after Deino restarts.
- If every delivery fails for a day, or the endpoint answers `410 Gone`,
  Deino turns the webhook off and says so on the page. Fix the URL and turn
  it on again.
- **Send test** is sent once and never retried.

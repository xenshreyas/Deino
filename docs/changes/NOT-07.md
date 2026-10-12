# NOT-07 · Suggested follow-up emails

## Progress log (PARITY_PLAN §7.1)
| NOT-07 Suggested follow-up emails | ☑️ done on branch `feat/NOT-07-followups` (scope cut by D12: no sending from Deino; `follow_up_drafts` table (one row per meeting: `ready`, `quiet` with a typed reason, or `dismissed`, which is permanent and clears the text; an edited draft is never replaced by a re-run); `deino-llm::followup` (`FollowUpDecision` with `FollowUpVerdict` send / colleague_owns / moved_to_chat / blocker_first / nothing_to_follow_up, `FollowUpDraftAnswer`, `placeholders`) and the `FollowUpDecideSystem`/`FollowUpDraftSystem`/`FollowUpUser` prompts on the `followup.email` route; `deino-core::followups`: `judge` (≥ 1 external attendee by `profile::InternalDomains` over ORG-04's people and companies, ≤ 10 people, no model call otherwise), `FollowUpInputs` (NOT-02's `MaterialSources`, so the transcript is redacted like notes; the enhanced notes, else the user's, through the same `TextRedactor`, NOT-09), `FollowUpGenerator` (decide first, draft only on `send`) behind the `FollowUpDrafter` port, `FollowUpService` (`suggested` for 3 days, `for_meeting`, `save`, `dismiss`, `open_in_mail` through `deino_export::mailto::compose`, `copy` the body); stage `follow_up_draft` at 98; plugin `followups` (6 commands); `features/followups`: Home's "Suggested follow-ups" through ORG-02's `HOME_SLOTS.useFollowUps`, the Notes tab's "Follow-up email ready · View draft" banner (dismiss asks once), the editor (To/Cc/Bcc pre-filled with the external attendees, subject, message with `[gaps]` highlighted and counted, Open in Mail, Copy message, "Send via Gmail or Outlook" disabled with a WIP badge); eval `followup.email` (`sales_discovery` must send with a placeholder, `colleague_owns` must stay quiet); Playwright walk-through with light and dark screenshots) | |

## Follow-ups (PARITY_PLAN §7.2)
- INT-07: "Send via Gmail or Outlook" with the 10 s undo (the button is in `features/followups/components/follow-up-editor.tsx`, disabled with a WIP badge); hide drafts once a reply exists in the thread (Granola §7.2).
- Later: a Settings switch for suggested follow-ups (Granola's "Suggested follow-up emails" toggle); today the stage always runs, and costs two `followup.email` calls only for qualifying meetings.
- Later: Home links to the meeting, where the banner opens the editor; a `?followUp=open` search param on the meeting route would open the editor straight from Home.
- FND-09: both calls use `followup.email` (smart tier). The decision is short; a fast-tier `followup.decide` route (with the `llm_calls.route` CHECK migration) would make it cheaper.
- Later: `insights::OwnDomains` (the user's domains from the settings) is used here, by insights and by briefs' rules; it belongs in `profile`.
- Later: a person without an address whose company domain is external still counts as internal for the "external attendee" rule (only addresses are judged).

## Architecture (ARCHITECTURE.md)
- Pipeline stages table: `follow_up_draft` (98, NOT-07): `deino_core::followups::FollowUpGenerator` behind `StagePorts::follow_ups` (`FollowUpDrafter`); ineligible, quiet, dismissed and edited meetings are `Skipped` with their reason.
- LLM routes: `followup.email` is now used, twice per qualifying meeting (decide, then draft), both structured.
- `deino-export::mailto`: `compose(&Compose { to, cc, bcc, subject, body })` encodes every address and field on its own (no header injection through `?`, `&`, `,` or line breaks); used by follow-ups.

## CLAUDE.md
- `crates/deino-core` row: add `followups` (NOT-07: suggested follow-up emails, the `follow_up_draft` stage's rules, decide then draft, Mail and the clipboard).

## Plan updates
- NOT-07 accept: [x] The draft is generated for the external sales fixture and not for the internal standup (`followups::acceptance_tests`, mock LLM).
- §7.2 NOT-05 row ("use `InternalDomains` for follow-up recipients and briefs"): the follow-up half is done.
- ORG-02's typed NOT-07 slot (`HOME_SLOTS.useFollowUps`) is filled.

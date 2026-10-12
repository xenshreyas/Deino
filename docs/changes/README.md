# Change notes

Each feature branch writes **one file here**, `docs/changes/<CARD-ID>.md`,
instead of editing the shared docs (`docs/PARITY_PLAN.md`,
`docs/ARCHITECTURE.md`, `CLAUDE.md`). Parallel branches therefore never
conflict on documentation. After each merge batch the merger folds every note
into the shared docs and deletes it.

Use these headings, and leave out any that don't apply:

```markdown
# <CARD-ID> · <card title>

## Progress log (PARITY_PLAN §7.1)
One row: what shipped, the ADR, the key types and commands.

## Follow-ups (PARITY_PLAN §7.2)
- <owner card or "Owner QA">: <what is left>

## Architecture (ARCHITECTURE.md)
- <section>: <the text to add or change>

## CLAUDE.md
- <the row or rule to add or change>

## Plan updates
- <acceptance boxes ticked, §7.2 rows this card closes>
```

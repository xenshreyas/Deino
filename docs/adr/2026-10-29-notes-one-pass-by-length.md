# Enhanced notes: one call whenever the transcript fits

Status: accepted (2026-10-29). Amends ADR 0030's "Long meetings: map-reduce".

**Context.** ADR 0030 noted any meeting over 30 minutes in ~20-minute parts
plus a merge, even when the transcript was small. A 52-minute weekly with a
27.7k-character transcript (about 7k tokens) became three part calls and a
merge. Each call is another chance for an answer to run away to the output
cap (12 of 77 `notes.enhance` calls since 2026-10-20 did, at about 3.6
minutes each), and the merge waits for the slowest part, so that regenerate
took about 4 minutes instead of about 20 seconds.

**Decision.** `transcript::parts` decides by length only: the whole
transcript goes in one call whenever it fits the target's character budget
(`HOSTED_PART_CHARS`, 120,000 characters or about 30k tokens, on a hosted
model; the context window on a local one). Duration no longer splits a
meeting. When a transcript is too long, it is still cut into ~20-minute
parts as before.

**Consequences.** Most meetings, up to roughly two to three hours of talk,
are written in one call: fewer calls, no merge step, and one model view of
the whole meeting. Very long transcripts keep map-reduce.

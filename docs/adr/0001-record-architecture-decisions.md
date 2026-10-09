# 0001. Record architecture decisions

- **Status:** Accepted
- **Date:** 2026-10-08
- **Card:** FND-02 (the first ADR); FND-13 owns the ADR backlog

## Context

Deino is built card by card by different agents and people
(`docs/PARITY_PLAN.md` §0). A decision that lives only in a PR description or
a chat log is lost to the next card, which then re-litigates it or quietly
breaks it. PARITY_PLAN §3A ("Documentation") asks for a short ADR for every
significant design choice: a library, a data model, an algorithm or a
cross-cutting pattern.

## Decision

Record each significant decision as a Markdown file in `docs/adr/`:

- Name it `NNNN-short-title.md`, numbered in order, starting from
  [`0000-template.md`](0000-template.md).
- Use the MADR-lite headings: Context, Decision, Alternatives, Consequences.
  Keep it to about one page.
- ADRs are append-only. To change a decision, write a new ADR and mark the old
  one "Superseded by NNNN"; do not rewrite history.
- The PR that implements a decision adds its ADR, and `CLAUDE.md` or
  `docs/ARCHITECTURE.md` links to it where agents will look.

## Alternatives

- **PR descriptions only:** not discoverable from the code and easy to lose.
- **A single decisions table** (PARITY_PLAN §8): good for defaults, too terse
  for the reasoning and trade-offs behind an architectural choice.

## Consequences

- Reviewers can reject a change that contradicts an accepted ADR without
  re-arguing it.
- Writing an ADR costs a few minutes per significant decision.
- FND-13 adds ADRs for the decisions already taken in PARITY_PLAN §4.0.

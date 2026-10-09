<!--
Title: a Conventional Commit stating the outcome, e.g. "feat(meetings): rename meetings from the sidebar".
Write this yourself: intent and evidence, not a file-by-file recap. Review standards: docs/REVIEW.md.
-->

## What

<!-- The card (e.g. FND-13) and what this PR changes, in two or three sentences. -->

## Why

<!-- The problem, or the PARITY_PLAN section this delivers. Name the reference project you benchmarked against and how Deino compares. -->

## How tested

<!-- Commands you ran and their results; new tests and the regression each prevents; manual QA on real hardware (audio, permissions). -->

- [ ] `scripts/clippy.sh`, `cargo fmt --all --check`, `cargo test --workspace`
- [ ] `cd frontend && pnpm typecheck && pnpm lint && pnpm test && pnpm build`
- [ ] Bindings regenerated (`cargo test -p deino --lib bindings`) and committed
- [ ] Manual QA:

## Screenshots

<!-- Light and dark for any UI change (§3B). "N/A" with one line on why nothing visible changed. -->

## Docs updated

- [ ] `docs/ARCHITECTURE.md` (processes, crates, modules, data flow, schema, extension points)
- [ ] ADR in `docs/adr/` for a significant decision
- [ ] Crate README, `CLAUDE.md`, `docs/setup/<provider>.md`, as relevant
- [ ] Legacy lists only shrank (`legacy_lints.rs`, `scripts/legacy-rust-files.txt`, `frontend/legacy-files.mjs`)

## Follow-ups

<!-- Gaps found that belong to another card, with the card ID. -->

# 0049. Stats on read, an estimated cost, and a year in review kept once

- **Status:** Accepted
- **Date:** 2026-10-12
- **Card:** ORG-09 (Stats and year in review)

## Context

The Stats page shows, for a chosen period, how many meetings the user had,
how long they recorded, whom they met (internal or external), when meetings
happen, a year-long activity heatmap, and what the LLM gateway's calls cost.
Three questions needed an answer:

1. **Store aggregates or compute them?** §4.0 says compute on read with good
   indexes unless a benchmark says otherwise.
2. **What counts as a meeting?** Trashed meetings (ORG-07), scheduled ones
   that never happened, and the bundled sample (FND-11) must not count, and
   every stats query must agree.
3. **Where does cost come from?** `llm_calls` (FND-09) stores tokens, never
   money.

The year in review must use the gateway on aggregated numbers and titles
only (never transcripts), open in December, and be generated once.

## Decision

- **Everything is computed on read.** Four compile-checked queries (counted
  meetings, their attendance, LLM usage grouped by provider and model, a
  year's titles) feed pure Rust aggregation. Local weekdays, hours and days
  come from the `LocalZone` port, so daylight-saving changes and tests are
  exact. 2,000 meetings, 8,000 attendance rows and 20,000 LLM calls take
  about 45 ms for the whole page in a release build (`stats_bench.rs`, load
  around 100), so no cache is needed.
- **One definition of a counted meeting:** the SQL view `stats_meetings`
  (`deleted_at IS NULL`, `status <> 'scheduled'`, not `meeting-sample`,
  started at `coalesce(started_at, created_at)`). A new exclusion is one line
  in one place. Range filters compare with `julianday()` because legacy rows
  spell timestamps differently.
- **Cost is an estimate from a static list-price table** (`pricing.rs`, per
  million tokens, integer micro-dollars). Local providers cost 0; a model
  without a known price is reported as unpriced and left out of the total,
  never guessed. The UI labels every amount as an estimate.
- **The recap has its own route, `recap`** (smart tier), which widens the
  `llm_calls.route` CHECK (a table rebuild). It is kept write-once in
  `year_recaps` (first write wins, so two racing generations agree) and
  opens in December or with `DEINO_RECAP_ANYTIME=1`.

## Alternatives

- **Cached counters on `people` and `meetings`:** faster in theory, but every
  merge, trash and attendance change would have to keep them right, and the
  benchmark shows no need.
- **A `cost_micros` column on `llm_calls`, written by the gateway:** freezes
  prices at call time (more accurate after a price change) but needs the
  gateway to own prices for every provider, including ones whose prices are
  unknown. The table can move there later without changing the page.
- **Reusing `brief` for the recap:** no migration, but the usage table
  would mislabel the recap's calls.

## Consequences

- A price change is one line in `pricing.rs`; OpenAI, Groq and OpenRouter
  models stay unpriced until someone adds theirs.
- Cards that add routes after this one rebuild `llm_calls` again (as before).
- Regenerating a recap needs a new action (none today).

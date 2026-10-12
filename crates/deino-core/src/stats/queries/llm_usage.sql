-- LLM calls started at or after ?1 (every call when NULL), summed per
-- provider and model, most calls first. The gateway writes
-- `llm_calls.started_at` in one RFC 3339 spelling and ?1 is bound the same
-- way, so the text comparison is exact and can use idx_llm_calls_started_at.
SELECT
    provider AS "provider!: ProviderId",
    model AS "model!",
    count(*) AS "calls!: u32",
    coalesce(sum(outcome = 'error'), 0) AS "failed!: u32",
    coalesce(sum(input_tokens), 0) AS "input_tokens!: i64",
    coalesce(sum(output_tokens), 0) AS "output_tokens!: i64"
FROM llm_calls
WHERE ?1 IS NULL OR started_at >= ?1
GROUP BY provider, model
ORDER BY 3 DESC, provider, model

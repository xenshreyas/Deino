-- Every exemplar made by the embedding model ?1 of a person not merged into
-- another, by person, best first. A person keeps at most five, so the scan
-- is small; idx_voiceprints_person_id orders it.
SELECT
    v.person_id AS "person_id: PersonId",
    p.is_me AS "is_me: bool",
    v.embedding
FROM voiceprints AS v
JOIN people AS p ON p.id = v.person_id
WHERE v.embedding_model = ?1 AND p.merged_into IS NULL
ORDER BY v.person_id, v.quality DESC, v.created_at DESC, v.id

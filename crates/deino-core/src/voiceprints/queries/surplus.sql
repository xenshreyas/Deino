-- The exemplars of person ?1 beyond the best ?2: lowest quality first to go,
-- the oldest among equals. idx_voiceprints_person_id serves it.
SELECT id AS "id: VoiceprintId"
FROM voiceprints
WHERE person_id = ?1
ORDER BY quality DESC, created_at DESC, id DESC
LIMIT -1 OFFSET ?2

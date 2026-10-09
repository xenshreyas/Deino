-- FND-02: drop the unused `licensing` table.
-- It was a remnant of the upstream PRO licence check; no code reads or writes it.
DROP TABLE IF EXISTS licensing;

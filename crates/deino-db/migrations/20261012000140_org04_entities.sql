-- ORG-04: people and companies as entities the user can merge, summarise and
-- recognise by their logo.
--
-- Companies merge the way people do (FND-07's `people.merged_into`): the
-- absorbed company stays as a hidden row whose `merged_into` names the one
-- that absorbed it, so its domain keeps resolving to the survivor and a
-- later calendar sync never recreates it. `EntityMerger::merge_companies`
-- moves every link in one transaction and flattens chains, so a merged row
-- always points straight at a listed company.
--
-- No aggregate is denormalised here. A person's last meeting and meeting
-- count (and a company's, over its people) are computed when the lists are
-- read, from `meeting_attendees` through idx_meeting_attendees_person_id and
-- the meetings primary key (ADR 0036 measures it and says why: attendance
-- has many writers, and trash, restore and status changes would all have to
-- keep a copy in step).

ALTER TABLE companies ADD COLUMN merged_into TEXT REFERENCES companies (id) ON DELETE SET NULL
    CHECK (merged_into IS NULL OR merged_into <> id);

-- When `summary_md` was last refreshed by the `update_entities` stage.
ALTER TABLE companies ADD COLUMN summary_updated_at TEXT;

-- When the favicon at `https://<domain>/favicon.ico` was last asked for,
-- whether or not one came back: logos are fetched once, never retried on
-- every visit (`LogoService`).
ALTER TABLE companies ADD COLUMN logo_checked_at TEXT;

-- The companies list: unmerged companies by name. Partial, like
-- idx_people_listed_name; idx_companies_name keeps serving lookups by name.
CREATE INDEX idx_companies_listed_name ON companies (name COLLATE NOCASE, id) WHERE merged_into IS NULL;
-- The foreign key; partial because only merged rows set it.
CREATE INDEX idx_companies_merged_into ON companies (merged_into) WHERE merged_into IS NOT NULL;

-- The egress log learns the `favicons` service (company logos). Its CHECK
-- lists every `ServiceId` (FND-12), and SQLite cannot change a CHECK in
-- place, so the table is rebuilt with its rows. Ids are kept, so the
-- newest-first order and the pruning order do not change.
CREATE TABLE egress_log_new (
    id INTEGER PRIMARY KEY,
    at TEXT NOT NULL,
    service TEXT NOT NULL CHECK (service IN (
        'llm_anthropic', 'llm_openai', 'llm_groq', 'llm_openrouter', 'llm_custom',
        'llm_ollama', 'hf_models', 'google_calendar', 'google_drive', 'gmail',
        'microsoft_graph', 'slack', 'notion', 'linear', 'todoist', 'asana',
        'hubspot', 'attio', 'salesforce', 'jira', 'monday', 'clickup', 'trello',
        'affinity', 'zoho_crm', 'web_search', 'webhooks', 'favicons'
    )),
    host TEXT NOT NULL CHECK (host <> '' AND instr(host, '/') = 0 AND instr(host, '?') = 0),
    method TEXT NOT NULL CHECK (method IN ('GET', 'HEAD', 'POST', 'PUT', 'PATCH', 'DELETE')),
    status INTEGER CHECK (status BETWEEN 100 AND 599),
    outcome TEXT NOT NULL CHECK (outcome IN ('sent', 'failed', 'blocked')),
    bytes_up INTEGER NOT NULL CHECK (bytes_up >= 0),
    bytes_down INTEGER NOT NULL CHECK (bytes_down >= 0)
) STRICT;

INSERT INTO egress_log_new (id, at, service, host, method, status, outcome, bytes_up, bytes_down)
SELECT id, at, service, host, method, status, outcome, bytes_up, bytes_down FROM egress_log ORDER BY id;

DROP TABLE egress_log;
ALTER TABLE egress_log_new RENAME TO egress_log;

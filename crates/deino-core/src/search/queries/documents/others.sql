-- People (not merged), companies and folders: the documents that belong to
-- no meeting. The same rules as their triggers.
INSERT INTO search_documents (kind, ref_id, title, body)
SELECT 'person', id, name, trim(coalesce(email, '') || ' ' || coalesce(title, '')) FROM people
WHERE merged_into IS NULL
UNION ALL
SELECT 'company', id, name, coalesce(domain, '') FROM companies
UNION ALL
SELECT 'folder', id, name, description FROM folders

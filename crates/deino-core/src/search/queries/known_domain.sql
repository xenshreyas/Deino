-- Whether anyone's email, or any company, uses the domain ?1 (or a
-- subdomain of it), so a typed `acme.com` is a filter and not a word.
SELECT EXISTS (
    SELECT 1 FROM people WHERE email LIKE '%@' || ?1 OR email LIKE '%.' || ?1
    UNION ALL
    SELECT 1 FROM companies WHERE domain = ?1 OR domain LIKE '%.' || ?1
) AS "known!: bool"

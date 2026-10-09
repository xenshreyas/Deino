SELECT
    at AS "at!: DateTime<Utc>",
    service AS "service!: ServiceId",
    host,
    method AS "method!: Method",
    status,
    outcome AS "outcome!: EgressOutcome",
    bytes_up,
    bytes_down
FROM egress_log
ORDER BY id DESC
LIMIT ?1

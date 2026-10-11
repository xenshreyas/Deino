-- One version (?2) of a meeting's (?1) notes, by the primary key.
SELECT
    meeting_id AS "meeting_id!: MeetingId",
    version AS "version!: u32",
    doc_json,
    markdown,
    template_id AS "template_id: TemplateId",
    model,
    basis_json AS "basis: Json<EnhanceBasis>",
    is_user_edited AS "is_user_edited!: bool",
    created_at AS "created_at!: DateTime<Utc>",
    updated_at AS "updated_at!: DateTime<Utc>"
FROM enhanced_notes
WHERE meeting_id = ?1 AND version = ?2

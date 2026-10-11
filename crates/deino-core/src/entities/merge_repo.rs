//! [`EntityMerger`] over SQLite: one transaction per merge.
//!
//! The absorbed row is written first, so an address or the "Me" flag it
//! hands over is free before the survivor claims it (both are unique).
//! Every link then moves in the same transaction; a failure anywhere rolls
//! the whole merge back.

use async_trait::async_trait;
use deino_db::DbError;
use sqlx::SqliteConnection;

use super::{CompanyMerge, EntityMerger, PersonMerge, SqliteEntityRepo};
use crate::companies::Company;
use crate::people::Person;
use crate::voiceprints::MAX_VOICEPRINTS_PER_PERSON;

#[async_trait]
impl EntityMerger for SqliteEntityRepo {
    async fn merge_people(&self, merge: &PersonMerge) -> Result<(), DbError> {
        let mut transaction = self.database.writer().begin().await?;
        write_person(&mut transaction, &merge.absorbed).await?;
        write_person(&mut transaction, &merge.survivor).await?;
        move_person_links(&mut transaction, merge).await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn merge_companies(&self, merge: &CompanyMerge) -> Result<(), DbError> {
        let mut transaction = self.database.writer().begin().await?;
        write_company(&mut transaction, &merge.absorbed).await?;
        write_company(&mut transaction, &merge.survivor).await?;
        let (from, to) = (&merge.absorbed.id, &merge.survivor.id);
        sqlx::query!(
            "UPDATE people SET company_id = ?1 WHERE company_id = ?2",
            to,
            from
        )
        .execute(&mut *transaction)
        .await?;
        sqlx::query!(
            "UPDATE companies SET merged_into = ?1 WHERE merged_into = ?2",
            to,
            from
        )
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }
}

/// Moves attendance, speakers, assignments, mentions, voices and earlier
/// merges from the absorbed person to the survivor.
async fn move_person_links(
    connection: &mut SqliteConnection,
    merge: &PersonMerge,
) -> Result<(), DbError> {
    let (from, to) = (&merge.absorbed.id, &merge.survivor.id);
    move_attendance(connection, merge).await?;
    sqlx::query!(
        "UPDATE speakers SET person_id = ?1 WHERE person_id = ?2",
        to,
        from
    )
    .execute(&mut *connection)
    .await?;
    sqlx::query!(
        "UPDATE action_items SET assignee_person_id = ?1 WHERE assignee_person_id = ?2",
        to,
        from,
    )
    .execute(&mut *connection)
    .await?;
    move_mentions(connection, merge).await?;
    move_voices(connection, merge).await?;
    sqlx::query!(
        "UPDATE people SET merged_into = ?1 WHERE merged_into = ?2",
        to,
        from
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Moves the absorbed person's meeting attendance to the survivor. A
/// meeting both attended keeps the survivor's row, upgraded from
/// `speaker_only` when the absorbed person was invited.
async fn move_attendance(
    connection: &mut SqliteConnection,
    merge: &PersonMerge,
) -> Result<(), DbError> {
    let (from, to) = (&merge.absorbed.id, &merge.survivor.id);
    sqlx::query!(
        "INSERT INTO meeting_attendees (meeting_id, person_id, role, rsvp)
         SELECT meeting_id, ?1, role, rsvp FROM meeting_attendees WHERE person_id = ?2
         ON CONFLICT (meeting_id, person_id) DO UPDATE SET
             role = CASE meeting_attendees.role WHEN 'speaker_only' THEN excluded.role
                    ELSE meeting_attendees.role END,
             rsvp = coalesce(meeting_attendees.rsvp, excluded.rsvp)",
        to,
        from,
    )
    .execute(&mut *connection)
    .await?;
    sqlx::query!("DELETE FROM meeting_attendees WHERE person_id = ?1", from)
        .execute(&mut *connection)
        .await?;
    Ok(())
}

/// Points @mentions at the survivor, in the user's notes and in every
/// version of the enhanced notes (NOT-02). A mention stores the person's
/// UUID in the opaque TipTap JSON (`attrs.id`); a 36-character UUID never
/// occurs there by chance, so replacing the text is exact. The chip keeps
/// the label it was written with, and the Markdown copies hold labels only.
async fn move_mentions(
    connection: &mut SqliteConnection,
    merge: &PersonMerge,
) -> Result<(), DbError> {
    let (from, to) = (&merge.absorbed.id, &merge.survivor.id);
    sqlx::query!(
        "UPDATE meeting_notes SET notes_json = replace(notes_json, ?2, ?1)
         WHERE instr(notes_json, ?2) > 0",
        to,
        from,
    )
    .execute(&mut *connection)
    .await?;
    sqlx::query!(
        "UPDATE enhanced_notes SET doc_json = replace(doc_json, ?2, ?1)
         WHERE instr(doc_json, ?2) > 0",
        to,
        from,
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Moves the absorbed person's voice profile (CAP-04): their voiceprints,
/// keeping the survivor's best [`MAX_VOICEPRINTS_PER_PERSON`] (the cap
/// `VoiceprintRepo::enrol` keeps), and any open "Is this …?" suggestion.
async fn move_voices(
    connection: &mut SqliteConnection,
    merge: &PersonMerge,
) -> Result<(), DbError> {
    let (from, to) = (&merge.absorbed.id, &merge.survivor.id);
    let keep = i64::try_from(MAX_VOICEPRINTS_PER_PERSON).unwrap_or(i64::MAX);
    sqlx::query!(
        "UPDATE voiceprints SET person_id = ?1 WHERE person_id = ?2",
        to,
        from
    )
    .execute(&mut *connection)
    .await?;
    sqlx::query!(
        "DELETE FROM voiceprints WHERE person_id = ?1 AND id NOT IN (
             SELECT id FROM voiceprints WHERE person_id = ?1
             ORDER BY quality DESC, created_at DESC LIMIT ?2)",
        to,
        keep,
    )
    .execute(&mut *connection)
    .await?;
    sqlx::query!(
        "UPDATE speaker_suggestions SET person_id = ?1 WHERE person_id = ?2",
        to,
        from
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Saves every field of `person` a merge may change.
async fn write_person(connection: &mut SqliteConnection, person: &Person) -> Result<(), DbError> {
    sqlx::query!(
        "UPDATE people
         SET email = ?1, company_id = ?2, title = ?3, avatar_path = ?4, notes_md = ?5,
             summary_md = ?6, summary_updated_at = ?7, is_me = ?8, merged_into = ?9, updated_at = ?10
         WHERE id = ?11",
        person.email,
        person.company_id,
        person.title,
        person.avatar_path,
        person.notes_md,
        person.summary_md,
        person.summary_updated_at,
        person.is_me,
        person.merged_into,
        person.updated_at,
        person.id,
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Saves every field of `company` a merge may change.
async fn write_company(
    connection: &mut SqliteConnection,
    company: &Company,
) -> Result<(), DbError> {
    sqlx::query!(
        "UPDATE companies
         SET domain = ?1, logo_path = ?2, logo_checked_at = ?3, notes_md = ?4, summary_md = ?5,
             summary_updated_at = ?6, merged_into = ?7, updated_at = ?8
         WHERE id = ?9",
        company.domain,
        company.logo_path,
        company.logo_checked_at,
        company.notes_md,
        company.summary_md,
        company.summary_updated_at,
        company.merged_into,
        company.updated_at,
        company.id,
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

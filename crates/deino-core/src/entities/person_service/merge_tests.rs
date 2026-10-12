//! ORG-04's second acceptance check: merging a person moves every link to
//! the survivor, one link type at a time, over the real schema.

use deino_domain::{AttendeeRole, Email, PersonId};

use super::*;
use crate::entities::testing::{NOW, World};

/// Three people: the survivor, the person merged into it, and an older
/// duplicate already merged into the absorbed one.
struct Merge {
    world: World,
    survivor: Person,
    absorbed: Person,
    earlier: Person,
}

impl Merge {
    /// The people, before any link.
    async fn new() -> Self {
        let world = World::new().await;
        let survivor = world.person("Priya Nair", None).await;
        let absorbed = world.person("Priya", Some("priya@deino.dev")).await;
        let earlier = world.person("P. Nair", None).await;
        let ids = [absorbed.id.to_string(), earlier.id.to_string()];
        world
            .run(
                "UPDATE people SET merged_into = ?1 WHERE id = ?2",
                &[&ids[0], &ids[1]],
            )
            .await;
        Self {
            world,
            survivor,
            absorbed,
            earlier,
        }
    }

    fn ids(&self) -> (String, String) {
        (self.survivor.id.to_string(), self.absorbed.id.to_string())
    }

    /// Both attended `both`; only the absorbed person organised `only`.
    async fn attendance(&self) {
        let both = self
            .world
            .meeting("both", "Planning", "2026-10-05T10:00:00Z")
            .await;
        let only = self
            .world
            .meeting("only", "Standup", "2026-10-08T10:00:00Z")
            .await;
        let world = &self.world;
        world
            .attend(&both, &self.survivor.id, AttendeeRole::SpeakerOnly)
            .await;
        world
            .attend(&both, &self.absorbed.id, AttendeeRole::Invitee)
            .await;
        world
            .attend(&only, &self.absorbed.id, AttendeeRole::Organizer)
            .await;
    }

    /// The absorbed person is a speaker, an assignee and an @mention in `only`.
    async fn speaker_item_and_mention(&self) {
        let (_, a) = self.ids();
        let world = &self.world;
        world
            .run(
                "INSERT INTO speakers (id, meeting_id, label, person_id, color)
                 VALUES ('01929999-0000-7000-8000-0000000000b1', 'only', 'Speaker 2', ?1, 2)",
                &[&a],
            )
            .await;
        world
            .run(
                "INSERT INTO action_items (id, title, assignee_person_id, status, created_by,
                    created_at, updated_at)
                 VALUES ('01929999-0000-7000-8000-0000000000c1', 'Rotate keys', ?1, 'open', 'ai', ?2, ?2)",
                &[&a, NOW],
            )
            .await;
        let mention = format!(r#"{{"type":"mention","attrs":{{"id":"{a}","label":"Priya"}}}}"#);
        world
            .run(
                "INSERT INTO meeting_notes (meeting_id, notes_markdown, notes_json, created_at, updated_at)
                 VALUES ('only', '@Priya', ?1, ?2, ?2)",
                &[&mention, NOW],
            )
            .await;
    }

    /// Merges the absorbed person into the survivor.
    async fn merge(&self) -> Person {
        let service = PersonService::new(self.world.ports());
        service
            .merge(&self.survivor.id, &self.absorbed.id)
            .await
            .unwrap()
    }

    async fn text(&self, sql: &str, bind: &str) -> Option<String> {
        self.world.text(sql, &[bind]).await
    }
}

#[tokio::test]
async fn attendance_moves_and_a_shared_meeting_keeps_the_stronger_role() {
    let merge = Merge::new().await;
    merge.attendance().await;
    let (s, a) = merge.ids();

    merge.merge().await;

    let roles = merge
        .text(
            "SELECT group_concat(meeting_id || ':' || role, ',') FROM (SELECT meeting_id, role
             FROM meeting_attendees WHERE person_id = ?1 ORDER BY meeting_id)",
            &s,
        )
        .await;
    assert_eq!(roles.as_deref(), Some("both:invitee,only:organizer"));
    let left = merge
        .text(
            "SELECT count(*) || '' FROM meeting_attendees WHERE person_id = ?1",
            &a,
        )
        .await;
    assert_eq!(left.as_deref(), Some("0"));
}

#[tokio::test]
async fn speakers_action_items_and_mentions_move() {
    let merge = Merge::new().await;
    merge.attendance().await;
    merge.speaker_item_and_mention().await;
    let (s, a) = merge.ids();

    merge.merge().await;

    let speaker = merge
        .text(
            "SELECT person_id FROM speakers WHERE label = ?1",
            "Speaker 2",
        )
        .await;
    assert_eq!(speaker.as_deref(), Some(s.as_str()));
    let assignee = merge
        .text(
            "SELECT assignee_person_id FROM action_items WHERE title = ?1",
            "Rotate keys",
        )
        .await;
    assert_eq!(assignee.as_deref(), Some(s.as_str()));
    let notes = merge
        .text(
            "SELECT notes_json FROM meeting_notes WHERE meeting_id = ?1",
            "only",
        )
        .await
        .unwrap();
    assert!(notes.contains(&s) && !notes.contains(&a), "{notes}");
}

#[tokio::test]
async fn mentions_in_enhanced_notes_move_too() {
    let merge = Merge::new().await;
    merge.attendance().await;
    let (s, a) = merge.ids();
    let doc = format!(
        r#"{{"type":"doc","content":[{{"type":"paragraph","content":[
            {{"type":"mention","attrs":{{"id":"{a}","label":"Priya"}}}}]}}]}}"#
    );
    let insert = "INSERT INTO enhanced_notes (meeting_id, version, doc_json, markdown, model,
        created_at, updated_at) VALUES ('only', 1, ?1, '@Priya', 'mock', ?2, ?2)";
    merge.world.run(insert, &[&doc, NOW]).await;

    merge.merge().await;

    let sql = "SELECT doc_json FROM enhanced_notes WHERE meeting_id = ?1";
    let stored = merge.text(sql, "only").await.unwrap();
    assert!(stored.contains(&s) && !stored.contains(&a), "{stored}");
}

#[tokio::test]
async fn the_address_and_earlier_merges_point_at_the_survivor() {
    let merge = Merge::new().await;

    let survivor = merge.merge().await;

    assert_eq!(
        survivor.email,
        Some(Email::parse("priya@deino.dev").unwrap())
    );
    let earlier = merge.world.stored_person(&merge.earlier.id).await;
    assert_eq!(earlier.merged_into, Some(merge.survivor.id));
    let gone = merge.world.stored_person(&merge.absorbed.id).await;
    assert_eq!(
        (gone.merged_into, gone.email),
        (Some(merge.survivor.id), None)
    );
}

/// Stores a voiceprint `n` of `person` with `quality`.
async fn voiceprint(world: &World, n: u8, person: &PersonId, quality: &str) {
    let id = format!("01929999-0000-7000-8000-00000000000{n}");
    world
        .run(
            "INSERT INTO voiceprints (id, person_id, embedding, embedding_model, quality, created_at)
             VALUES (?1, ?2, zeroblob(1024), 'wespeaker', CAST(?3 AS REAL), ?4)",
            &[&id, &person.to_string(), quality, NOW],
        )
        .await;
}

/// A speaker in `call` that Deino suggests is `person`.
async fn suggestion(world: &World, person: &PersonId) {
    world.meeting("call", "Call", "2026-10-08T10:00:00Z").await;
    world
        .run(
            "INSERT INTO speakers (id, meeting_id, label, color)
             VALUES ('01929999-0000-7000-8000-0000000000b9', 'call', 'Speaker 3', 3)",
            &[],
        )
        .await;
    world
        .run(
            "INSERT INTO speaker_suggestions (speaker_id, person_id, score)
             VALUES ('01929999-0000-7000-8000-0000000000b9', ?1, 0.6)",
            &[&person.to_string()],
        )
        .await;
}

#[tokio::test]
async fn the_voice_profile_moves_keeping_the_best_five_voiceprints() {
    let merge = Merge::new().await;
    let (survivor, absorbed) = (merge.survivor.id, merge.absorbed.id);
    for (n, owner, quality) in [
        (1, survivor, "0.5"),
        (2, survivor, "0.5"),
        (3, survivor, "0.5"),
    ] {
        voiceprint(&merge.world, n, &owner, quality).await;
    }
    for (n, owner, quality) in [
        (4, absorbed, "0.9"),
        (5, absorbed, "0.8"),
        (6, absorbed, "0.1"),
    ] {
        voiceprint(&merge.world, n, &owner, quality).await;
    }
    suggestion(&merge.world, &absorbed).await;
    let (s, _) = merge.ids();

    merge.merge().await;

    let kept = merge
        .text(
            "SELECT group_concat(substr(id, 36), '') FROM
                (SELECT id FROM voiceprints WHERE person_id = ?1 ORDER BY id)",
            &s,
        )
        .await;
    assert_eq!(kept.as_deref(), Some("12345"), "the weakest (6) is evicted");
    let suggested = merge
        .text(
            "SELECT person_id FROM speaker_suggestions WHERE score > ?1",
            "0",
        )
        .await;
    assert_eq!(suggested.as_deref(), Some(s.as_str()));
}

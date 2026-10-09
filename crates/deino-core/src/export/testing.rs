//! Export test helpers: ports over a seeded database, in a fixed zone.

use std::sync::Arc;

use chrono::FixedOffset;
use deino_db::{Database, test_db};
use deino_domain::Email;
use deino_testkit::at;
use deino_testkit::fixture::{USER_EMAIL, seed_fixtures};

use super::{ExportAssembler, ExportPorts, FixedZone};
use crate::people::{MeRepo, PersonRepo, SqlitePeopleRepo};

/// The standup fixture's ID.
pub(crate) const STANDUP: &str = "meeting-fixture-standup";

/// Every export port over `database`, showing times at UTC−07:00.
pub(crate) fn sqlite_ports(database: &Database) -> ExportPorts {
    let zone = FixedZone(FixedOffset::west_opt(7 * 3_600).expect("a valid offset"));
    ExportPorts::sqlite(database, Arc::new(zone))
}

/// A database with the eight fixture meetings, the user marked as "Me" (as
/// the profile does in the app), and an assembler over it.
pub(crate) async fn seeded_assembler() -> (Database, ExportAssembler) {
    let database = test_db().await;
    seed_fixtures(&database).await.expect("the fixtures seed");
    let people = SqlitePeopleRepo::new(database.clone());
    let email = Email::parse(USER_EMAIL).expect("the user's email is valid");
    let user = people
        .find_by_email(&email)
        .await
        .expect("people read")
        .expect("the user is seeded");
    people
        .make_me(&user.id, at("2026-10-09T09:00:00Z"))
        .await
        .expect("the user becomes Me");
    let assembler = ExportAssembler::new(sqlite_ports(&database));
    (database, assembler)
}

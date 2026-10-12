//! The library over the sample, for this module's tests: UTC, and "now"
//! is [`SAMPLE_NOW`].

use std::sync::Arc;

use deino_domain::{FakeClock, FixedZone};
use deino_testkit::{at, test_db};

use super::{Library, ReadContext, ReadPorts, SAMPLE_NOW, seed_sample_library};

/// The fixture standup (Thursday 8 October).
pub const STANDUP: &str = "meeting-fixture-standup";
/// The fixture sales call (30 September), tagged "Customer", in Customers › Brightkite.
pub const SALES: &str = "meeting-fixture-sales-discovery";
/// The fixture support call (5 October), tagged "Customer", in Customers.
pub const SUPPORT: &str = "meeting-fixture-customer-support";

/// The seeded sample's ports.
pub async fn ports() -> ReadPorts {
    let database = test_db().await;
    let zone = Arc::new(FixedZone::hours(0).unwrap());
    seed_sample_library(&database, zone.as_ref()).await.unwrap();
    let context = ReadContext {
        zone,
        clock: Arc::new(FakeClock::new(at(SAMPLE_NOW))),
        embedder: None,
        redactor: Arc::new(crate::redaction::NoRedaction),
    };
    ReadPorts::sqlite(&database, &context)
}

/// The library over the seeded sample.
pub async fn library() -> Library {
    Library::new(&ports().await)
}

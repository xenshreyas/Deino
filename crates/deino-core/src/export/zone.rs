//! The user's time zone, as a port: exports show local times, and tests
//! pin the zone so golden files do not depend on the machine.

use chrono::{DateTime, FixedOffset, Local, Offset, TimeZone, Utc};

/// Says what offset from UTC the user's clock had at an instant.
pub trait LocalZone: Send + Sync {
    /// The offset at `at` (daylight saving included).
    fn offset_at(&self, at: DateTime<Utc>) -> FixedOffset;

    /// `at` in the user's zone.
    fn local(&self, at: DateTime<Utc>) -> DateTime<FixedOffset> {
        at.with_timezone(&self.offset_at(at))
    }
}

/// The Mac's time zone.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemZone;

impl LocalZone for SystemZone {
    fn offset_at(&self, at: DateTime<Utc>) -> FixedOffset {
        Local.offset_from_utc_datetime(&at.naive_utc()).fix()
    }
}

/// One fixed offset, for tests.
#[derive(Clone, Copy, Debug)]
pub struct FixedZone(pub FixedOffset);

impl LocalZone for FixedZone {
    fn offset_at(&self, _at: DateTime<Utc>) -> FixedOffset {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fixed_zone_shifts_every_instant_alike() {
        let zone = FixedZone(FixedOffset::west_opt(7 * 3_600).unwrap());
        let at = DateTime::parse_from_rfc3339("2026-10-08T15:31:04Z")
            .unwrap()
            .to_utc();

        assert_eq!(zone.local(at).to_rfc3339(), "2026-10-08T08:31:04-07:00");
    }
}

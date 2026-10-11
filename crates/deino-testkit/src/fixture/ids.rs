//! Stable IDs for seeded fixture rows.
//!
//! Every row the seeder writes gets a UUID derived from a readable key
//! (`segment/meeting-fixture-standup/3`), so re-seeding rewrites the same
//! rows and tests and the mocked UI can name them. The UUID is two FNV-1a
//! hashes of the key with the RFC 9562 version 8 ("custom") and variant bits
//! set; `frontend/src/dev/mock-ipc/fixture-ids.ts` computes the same values.

/// FNV-1a's 64-bit offset basis.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a's 64-bit prime.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
/// Clears the version nibble (bits 12 to 15 of the high half's last group).
const VERSION_MASK: u64 = 0xffff_ffff_ffff_0fff;
/// Version 8: a custom, deterministic UUID.
const VERSION_8: u64 = 0x0000_0000_0000_8000;
/// Clears the two variant bits.
const VARIANT_MASK: u64 = 0x3fff_ffff_ffff_ffff;
/// The RFC 9562 variant (`10`).
const VARIANT_RFC: u64 = 0x8000_0000_0000_0000;

/// The fixture UUID for `key`, in hyphenated lower-case form.
pub fn fixture_uuid(key: &str) -> String {
    let high = (fnv1a(key.as_bytes()) & VERSION_MASK) | VERSION_8;
    let low = (fnv1a(format!("{key}#").as_bytes()) & VARIANT_MASK) | VARIANT_RFC;
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        high >> 32,
        (high >> 16) & 0xffff,
        high & 0xffff,
        low >> 48,
        low & 0xffff_ffff_ffff,
    )
}

/// The 64-bit FNV-1a hash of `bytes`.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(FNV_OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

/// The fixture calendar account every fixture event belongs to.
pub const CALENDAR_ACCOUNT_NAME: &str = "Deino fixtures";

/// The fixture calendar (in [`CALENDAR_ACCOUNT_NAME`]) every fixture event
/// belongs to.
pub const CALENDAR_NAME: &str = "Work";

/// The key of the fixture calendar's ID.
pub const CALENDAR_KEY: &str = "calendar/work";

/// The fixture calendar's colour (macOS Calendar's blue).
pub const CALENDAR_COLOR: &str = "#34AADC";

/// The key of a fixture company's ID.
pub fn company_key(domain: &str) -> String {
    format!("company/{domain}")
}

/// The key of a fixture person's ID.
pub fn person_key(email: &str) -> String {
    format!("person/{email}")
}

/// The key of a fixture meeting's calendar event ID.
pub fn event_key(meeting: &str) -> String {
    format!("event/{meeting}")
}

/// The key of a fixture speaker's ID.
pub fn speaker_key(meeting: &str, label: &str) -> String {
    format!("speaker/{meeting}/{label}")
}

/// The key of a fixture segment's ID.
pub fn segment_key(meeting: &str, index: usize) -> String {
    format!("segment/{meeting}/{index}")
}

#[cfg(test)]
mod tests {
    use deino_domain::SegmentId;

    use super::*;

    #[test]
    fn fixture_uuids_are_stable_version_8_uuids() {
        let id = fixture_uuid("segment/meeting-fixture-standup/0");

        let parsed: SegmentId = id.parse().unwrap();
        assert_eq!(parsed.as_uuid().get_version_num(), 8);
        assert_eq!(parsed.to_string(), id);
        // The TypeScript twin (fixture-ids.ts) asserts the same value.
        assert_eq!(id, "c7874d53-3bad-8a77-b822-b86e675a1cbc");
    }

    #[test]
    fn different_keys_give_different_ids() {
        assert_ne!(
            fixture_uuid("person/a@deino.dev"),
            fixture_uuid("person/b@deino.dev")
        );
    }
}

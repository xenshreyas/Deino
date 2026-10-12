//! [`StabilityTracker`]: a file is ready once it stops changing.
//!
//! A recorder or a copy writes a file over minutes; reading it early would
//! import half a meeting. Each scan reports what it saw; a file whose size
//! and modification time stayed the same for [`STABLE_FOR`] is reported
//! ready, once. If it changes again it starts over.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};

use super::Sighting;

/// How long a file must stay unchanged before it is imported.
pub const STABLE_FOR: Duration = Duration::from_secs(10);

/// What the tracker knows about one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Observation {
    size_bytes: u64,
    modified_ms: i64,
    /// When it was first seen with this size and modification time.
    since: DateTime<Utc>,
    /// Whether it was already reported ready.
    reported: bool,
}

/// The files of one folder and how long each has been unchanged.
#[derive(Clone, Debug, Default)]
pub struct StabilityTracker {
    files: HashMap<PathBuf, Observation>,
}

impl StabilityTracker {
    /// Records one scan of the folder, taken at `now`, and returns the files
    /// that just became ready. Files missing from the scan are forgotten.
    pub fn settled(&mut self, scan: &[Sighting], now: DateTime<Utc>) -> Vec<Sighting> {
        self.files
            .retain(|path, _| scan.iter().any(|sighting| sighting.path == *path));
        let stable_for = TimeDelta::from_std(STABLE_FOR).unwrap_or(TimeDelta::MAX);
        let mut ready = Vec::new();
        for sighting in scan {
            let seen = self.observe(sighting, now);
            if !seen.reported && now - seen.since >= stable_for {
                seen.reported = true;
                ready.push(sighting.clone());
            }
        }
        ready
    }

    /// The observation of `sighting`, restarted when the file changed.
    fn observe(&mut self, sighting: &Sighting, now: DateTime<Utc>) -> &mut Observation {
        let fresh = Observation {
            size_bytes: sighting.size_bytes,
            modified_ms: sighting.modified_ms,
            since: now,
            reported: false,
        };
        let seen = self.files.entry(sighting.path.clone()).or_insert(fresh);
        if seen.size_bytes != sighting.size_bytes || seen.modified_ms != sighting.modified_ms {
            *seen = fresh;
        }
        seen
    }
}

#[cfg(test)]
mod tests {
    use deino_testkit::at;

    use super::*;

    fn file(size_bytes: u64, modified_ms: i64) -> Sighting {
        Sighting {
            path: PathBuf::from("/watch/call.mp4"),
            size_bytes,
            modified_ms,
        }
    }

    fn at_second(second: u32) -> DateTime<Utc> {
        at("2026-10-12T10:00:00Z") + TimeDelta::seconds(i64::from(second))
    }

    #[test]
    fn a_file_is_ready_after_ten_unchanged_seconds_and_only_once() {
        let mut tracker = StabilityTracker::default();

        assert!(tracker.settled(&[file(10, 1)], at_second(0)).is_empty());
        assert!(tracker.settled(&[file(10, 1)], at_second(9)).is_empty());
        assert_eq!(
            tracker.settled(&[file(10, 1)], at_second(10)),
            [file(10, 1)]
        );
        assert!(tracker.settled(&[file(10, 1)], at_second(30)).is_empty());
    }

    #[test]
    fn a_growing_file_waits_until_it_stops_changing() {
        let mut tracker = StabilityTracker::default();

        tracker.settled(&[file(10, 1)], at_second(0));
        assert!(tracker.settled(&[file(20, 1)], at_second(10)).is_empty());
        assert!(tracker.settled(&[file(20, 2)], at_second(15)).is_empty());
        assert!(tracker.settled(&[file(20, 2)], at_second(24)).is_empty());
        assert_eq!(tracker.settled(&[file(20, 2)], at_second(25)).len(), 1);
    }

    #[test]
    fn a_file_that_comes_back_changed_is_ready_again() {
        let mut tracker = StabilityTracker::default();
        tracker.settled(&[file(10, 1)], at_second(0));
        tracker.settled(&[file(10, 1)], at_second(10));

        tracker.settled(&[], at_second(11));
        tracker.settled(&[file(30, 5)], at_second(12));

        assert_eq!(
            tracker.settled(&[file(30, 5)], at_second(22)),
            [file(30, 5)]
        );
    }
}

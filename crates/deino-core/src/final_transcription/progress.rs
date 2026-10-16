//! How far transcription has got: [`TrackProgress`], the callback a track
//! transcriber reports through, and [`TrackShares`], which turns each
//! track's progress into the whole run's.
//!
//! A fraction is `0.0..=1.0`. The run's fraction is
//! `(tracks done + the current track's fraction) / tracks`: every track
//! weighs the same, which matches their durations, because the microphone
//! and system tracks both span the whole meeting. Only numbers pass
//! through here, never what was said.

use std::fmt;
use std::sync::Arc;

/// Reports how far a piece of work has got, as a fraction from 0.0 to 1.0.
/// The default reports nowhere.
#[derive(Clone)]
pub struct TrackProgress(Arc<dyn Fn(f32) + Send + Sync>);

impl TrackProgress {
    /// Progress that calls `report` with each fraction. `report` must not
    /// block: it runs on the transcriber's thread.
    pub fn new(report: impl Fn(f32) + Send + Sync + 'static) -> Self {
        Self(Arc::new(report))
    }

    /// Reports `fraction`, clamped to `0.0..=1.0`.
    pub fn report(&self, fraction: f32) {
        (self.0)(fraction.clamp(0.0, 1.0));
    }
}

impl Default for TrackProgress {
    fn default() -> Self {
        Self::new(|_| {})
    }
}

impl fmt::Debug for TrackProgress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TrackProgress")
    }
}

/// `done` of `total` as a fraction: 1.0 when `total` is zero.
pub fn fraction_of(done: u64, total: u64) -> f32 {
    if total == 0 {
        return 1.0;
    }
    let permyriad = done.min(total).saturating_mul(10_000) / total;
    f32::from(u16::try_from(permyriad).unwrap_or(10_000)) / 10_000.0
}

/// A run over `tracks` tracks, reporting the whole run's fraction.
#[derive(Clone, Debug)]
pub(super) struct TrackShares {
    run: TrackProgress,
    tracks: usize,
}

impl TrackShares {
    /// Shares of `run` for `tracks` tracks, in order.
    pub(super) fn new(run: TrackProgress, tracks: usize) -> Self {
        Self { run, tracks }
    }

    /// The progress of track `index` (from 0), as part of the run.
    pub(super) fn track(&self, index: usize) -> TrackProgress {
        let shares = self.clone();
        TrackProgress::new(move |fraction| shares.report(index, fraction))
    }

    /// Track `index` is finished.
    pub(super) fn done(&self, index: usize) {
        self.report(index, 1.0);
    }

    fn report(&self, index: usize, fraction: f32) {
        let tracks = count(self.tracks.max(1));
        self.run.report((count(index) + fraction) / tracks);
    }
}

/// A track count as a float (a meeting has two or three tracks).
fn count(tracks: usize) -> f32 {
    f32::from(u16::try_from(tracks).unwrap_or(u16::MAX))
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    fn recorded() -> (TrackProgress, Arc<Mutex<Vec<f32>>>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&seen);
        let progress = TrackProgress::new(move |fraction| sink.lock().unwrap().push(fraction));
        (progress, seen)
    }

    #[test]
    fn two_tracks_each_fill_half_of_the_run() {
        let (run, seen) = recorded();
        let shares = TrackShares::new(run, 2);

        shares.track(0).report(0.5);
        shares.done(0);
        shares.track(1).report(0.4);
        shares.done(1);

        assert_eq!(*seen.lock().unwrap(), vec![0.25, 0.5, 0.7, 1.0]);
    }

    #[test]
    fn fractions_are_clamped_and_an_empty_total_is_done() {
        let (progress, seen) = recorded();

        progress.report(1.5);
        progress.report(-0.5);

        assert_eq!(*seen.lock().unwrap(), vec![1.0, 0.0]);
        assert!((fraction_of(1, 3) - 0.3333).abs() < 1e-4);
        assert!((fraction_of(5, 3) - 1.0).abs() < f32::EPSILON);
        assert!((fraction_of(0, 0) - 1.0).abs() < f32::EPSILON);
    }
}

//! Many lines at once: [`ALIGN_SESSIONS`] model sessions on their own
//! threads, and a [`LineMemory`] of lines already timed.
//!
//! A line's words depend only on its text, its span and the samples the
//! model reads for it (the span padded by [`PAD_MS`]), so a line a pass
//! during the meeting timed is the same line after it when all of those
//! match exactly (its key); anything else is timed again.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use deino_core::diarization::AlignLine;
use deino_core::segments::WordTiming;

use super::model::Aligner;
use super::{PAD_MS, align_line};
use crate::config::RuntimeConfig;
use crate::error::DiarizeError;
use crate::units::SAMPLE_RATE;

/// Model sessions that time lines at once (each on its own thread).
pub const ALIGN_SESSIONS: usize = 3;

/// A line's key: its span, its text and a fingerprint of what the model reads.
type LineKey = (u32, u32, String, u128);

/// Lines already timed, shared between passes over one recording.
#[derive(Default)]
pub struct LineMemory {
    lines: Mutex<HashMap<LineKey, Vec<WordTiming>>>,
}

impl LineMemory {
    /// Forgets every line (a new recording).
    pub fn clear(&self) {
        if let Ok(mut lines) = self.lines.lock() {
            lines.clear();
        }
    }
}

/// The words of every line of `samples`, from `memory` where it has them.
pub(super) fn time_lines(
    (model, runtime): (&std::path::Path, &RuntimeConfig),
    samples: &[f32],
    lines: &[AlignLine],
    memory: Option<&LineMemory>,
) -> Result<Vec<Vec<WordTiming>>, DiarizeError> {
    let Some(memory) = memory else {
        return in_sessions((model, runtime), samples, lines);
    };
    let keys: Vec<LineKey> = lines.iter().map(|line| key(samples, line)).collect();
    let mut timed: Vec<Option<Vec<WordTiming>>> = {
        let known = memory.lines.lock().map_err(|_| DiarizeError::Cancelled)?;
        keys.iter().map(|key| known.get(key).cloned()).collect()
    };
    let missing: Vec<usize> = (0..lines.len())
        .filter(|&at| timed.get(at).is_some_and(Option::is_none))
        .collect();
    let asked: Vec<AlignLine> = missing
        .iter()
        .filter_map(|&at| lines.get(at).cloned())
        .collect();
    let answers = in_sessions((model, runtime), samples, &asked)?;
    let mut known = memory.lines.lock().map_err(|_| DiarizeError::Cancelled)?;
    for (&at, words) in missing.iter().zip(answers) {
        if let (Some(slot), Some(key)) = (timed.get_mut(at), keys.get(at)) {
            known.insert(key.clone(), words.clone());
            *slot = Some(words);
        }
    }
    Ok(timed.into_iter().map(Option::unwrap_or_default).collect())
}

/// The key of `line` in `samples`.
fn key(samples: &[f32], line: &AlignLine) -> LineKey {
    let rate = SAMPLE_RATE as usize;
    let total_ms = samples.len() * 1000 / rate;
    let start = (line.start.0.saturating_sub(PAD_MS) as usize).min(total_ms);
    let end = (line.end.0.saturating_add(PAD_MS) as usize).min(total_ms);
    let read = samples
        .get(start * rate / 1000..end * rate / 1000)
        .unwrap_or_default();
    let (mut low, mut high) = (0xcbf2_9ce4_8422_2325_u64, 0x8422_2325_cbf2_9ce4_u64);
    for sample in read
        .iter()
        .map(|sample| u64::from(sample.to_bits()))
        .chain([end as u64])
    {
        low = (low ^ sample).wrapping_mul(0x0000_0100_0000_01b3);
        high = (high ^ sample.rotate_left(17)).wrapping_mul(0x0000_0001_0000_0193);
    }
    let print = (u128::from(high) << 64) | u128::from(low);
    (line.start.0, line.end.0, line.text.clone(), print)
}

/// Each line's words, or why it could not be timed, by line index.
type Slots = Vec<Option<Result<Vec<WordTiming>, DiarizeError>>>;

/// What the sessions share: the lines, a counter of the next one to take,
/// and the answers.
struct Work<'a> {
    samples: &'a [f32],
    lines: &'a [AlignLine],
    next: AtomicUsize,
    slots: Mutex<Slots>,
}

impl Work<'_> {
    /// Times lines with `aligner` until none are left.
    fn run(&self, aligner: &mut Aligner) {
        while let Some((at, line)) = self.take() {
            let result = align_line(aligner, self.samples, line);
            if let Some(slot) = self
                .slots
                .lock()
                .ok()
                .as_mut()
                .and_then(|slots| slots.get_mut(at))
            {
                *slot = Some(result);
            }
        }
    }

    fn take(&self) -> Option<(usize, &AlignLine)> {
        let at = self.next.fetch_add(1, Ordering::Relaxed);
        self.lines.get(at).map(|line| (at, line))
    }
}

/// The lines across up to [`ALIGN_SESSIONS`] sessions, each taking the next
/// line from a shared counter; results in line order.
fn in_sessions(
    (model, runtime): (&std::path::Path, &RuntimeConfig),
    samples: &[f32],
    lines: &[AlignLine],
) -> Result<Vec<Vec<WordTiming>>, DiarizeError> {
    if lines.is_empty() {
        return Ok(Vec::new());
    }
    let aligners = (0..ALIGN_SESSIONS.min(lines.len()))
        .map(|_| Aligner::load(model, runtime))
        .collect::<Result<Vec<_>, _>>()?;
    let work = Work {
        samples,
        lines,
        next: AtomicUsize::new(0),
        slots: Mutex::new((0..lines.len()).map(|_| None).collect()),
    };
    std::thread::scope(|scope| {
        for mut aligner in aligners {
            let work = &work;
            scope.spawn(move || work.run(&mut aligner));
        }
    });
    work.slots
        .into_inner()
        .map_err(|_| DiarizeError::Cancelled)?
        .into_iter()
        .map(|slot| slot.unwrap_or_else(|| Ok(Vec::new())))
        .collect()
}

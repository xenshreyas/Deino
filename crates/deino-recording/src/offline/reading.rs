//! Reading one utterance with the batch engine: its words placed on the
//! recording's timeline, and the halves of a reading that stopped early.

use std::ops::Range;

use deino_core::final_transcription::{HeardSpan, TrackError};
use deino_core::segments::WordTiming;
use deino_stt::words::{TimedWord, words_from_tokens};
use deino_stt::{BatchTranscriber, Millis};

use super::{CHECKED_UTTERANCE, MIN_WORDS_PER_SECOND};

/// The utterance `piece`, read again as two halves when the first reading
/// heard too few words for its length. Words are counted from their times,
/// so once the engine shows it times no words (`timed` false: text without
/// times) its halves could never hear more and are not asked for.
pub(super) fn checked(
    engine: &dyn BatchTranscriber,
    pcm: &[f32],
    piece: Range<Millis>,
    timed: &mut Option<bool>,
) -> Result<Vec<HeardSpan>, TrackError> {
    let whole = heard(engine, pcm, piece.clone())?;
    if let Some(span) = &whole {
        *timed = Some(!span.words.is_empty());
    }
    let length = piece.end.saturating_sub(piece.start);
    let words = whole.as_ref().map_or(0, |span| span.words.len());
    let enough =
        u64::try_from(words).unwrap_or(u64::MAX) >= MIN_WORDS_PER_SECOND * length.0 / 1_000;
    if length < CHECKED_UTTERANCE || enough || *timed == Some(false) {
        return Ok(whole.into_iter().collect());
    }
    let middle = Millis(piece.start.0 + length.0 / 2);
    let halves: Vec<HeardSpan> = [piece.start..middle, middle..piece.end]
        .into_iter()
        .map(|half| heard(engine, pcm, half))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();
    let half_words: usize = halves.iter().map(|span| span.words.len()).sum();
    Ok(if half_words > words {
        halves
    } else {
        whole.into_iter().collect()
    })
}

/// What the engine hears in `piece` of `pcm`, if anything.
fn heard(
    engine: &dyn BatchTranscriber,
    pcm: &[f32],
    piece: Range<Millis>,
) -> Result<Option<HeardSpan>, TrackError> {
    let samples = pcm.get(piece.start.to_samples()..piece.end.to_samples().min(pcm.len()));
    let transcript = engine
        .transcribe(samples.unwrap_or_default().to_vec())
        .map_err(|error| TrackError::Failed(Box::new(error)))?;
    let text = transcript.text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let words: Vec<WordTiming> = words_from_tokens(&transcript.tokens, piece.start)
        .iter()
        .map(timing)
        .collect();
    let start = words.first().map_or(stored(piece.start), |word| word.start);
    let end = words.last().map_or(stored(piece.end), |word| word.end);
    Ok(Some(HeardSpan {
        start,
        end: end.max(start),
        text: text.to_owned(),
        words,
    }))
}

fn timing(word: &TimedWord) -> WordTiming {
    WordTiming {
        text: word.text.clone(),
        start: stored(word.start),
        end: stored(word.end.max(word.start)),
    }
}

/// An engine time as a stored time (saturating past 49 days).
fn stored(time: Millis) -> deino_domain::Millis {
    deino_domain::Millis(u32::try_from(time.0).unwrap_or(u32::MAX))
}

//! Custom vocabulary and transcript corrections (TRN-05).
//!
//! The user keeps terms ("Shreyas", "Deino") with what they sound like
//! when misheard, and replacement rules ("gonna" → "going to"), in Settings
//! → Vocabulary. The names of people, companies and a meeting's invitees
//! join them automatically. Parakeet takes no hotwords (transcribe-cpp 0.3.1
//! sets `TRANSCRIBE_FEATURE_VOCABULARY` only for Whisper, Qwen3-ASR, Granite,
//! FunASR-Nano and MOSS), so the vocabulary corrects text after decoding:
//!
//! - **Matching** ([`Lexicon`], `matcher`): whole words only, ignoring case;
//!   misheard names by Jaro-Winkler (0.92 for names of several words, 0.95
//!   with stricter guards for one word).
//! - **Live** ([`LiveVocabulary`]): the recorder corrects each segment as it
//!   is saved and shown, and keeps the original.
//! - **After the meeting** ([`FixupService`], stage `vocab_fixups` at 40):
//!   everything again, with the meeting's invitees and newer entries.
//! - **Undo** ([`CorrectionRepo::undo`]): every correction keeps the
//!   engine's original in `vocab_corrections`; undoing restores it unless the
//!   user edited the segment since, and the stage never redoes it.
//! - **Notes** ([`PromptVocabulary`], NOT-02's `Vocabulary` port): the
//!   user's terms are listed in the enhanced-notes prompt.
//!
//! | File | Role |
//! |---|---|
//! | `model.rs` | `VocabTerm`, `TermInput` → `TermDraft`, `Correction`, `UndoReport` |
//! | `tokens.rs`, `lexicon.rs`, `matcher.rs`, `corrector.rs` | Pure matching and correction |
//! | `repo.rs`, `corrections.rs` | `vocab_terms`/`vocab_variants` and `vocab_corrections` |
//! | `names.rs` | People, companies and invitees |
//! | `service.rs`, `live.rs`, `fixups.rs`, `prompt.rs` | Entries, the live pass, the stage's rules, the notes' terms |

mod corrections;
mod corrector;
mod error;
mod fixups;
mod lexicon;
mod live;
mod matcher;
mod model;
mod names;
mod prompt;
mod repo;
mod service;
#[cfg(test)]
mod test_world;
mod tokens;

pub use corrections::{CorrectionRepo, SqliteCorrectionRepo, UndoScope};
pub use corrector::{Corrected, Corrector};
pub use error::VocabError;
pub use fixups::{FixupOutcome, FixupPorts, FixupService, corrections};
pub use lexicon::{KnownName, Lexicon, NameKind};
#[cfg(any(test, feature = "test-support"))]
pub use live::RecordedLiveVocabulary;
pub use live::{LiveVocabulary, NoLiveVocabulary};
pub use matcher::{
    MAX_LETTER_GAP, MIN_SINGLE_WORD_LETTERS, MULTI_WORD_SIMILARITY, SINGLE_WORD_SIMILARITY,
};
pub use model::{
    CorrectedSegment, Correction, MAX_TERM_CHARS, MAX_VARIANTS, TermDraft, TermInput, UndoReport,
    VocabTerm,
};
pub use names::NameSources;
pub use prompt::PromptVocabulary;
pub use repo::{SqliteVocabRepo, VocabRepo};
pub use service::{VocabularyPorts, VocabularyService};

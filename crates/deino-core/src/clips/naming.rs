//! Clip file names: safe on every file system, and unique in their folder.

use std::collections::HashSet;
use std::hash::BuildHasher;

/// The longest stem a clip file gets, in characters (well under the 255
/// bytes HFS+ and APFS allow, leaving room for " 99.m4a").
const MAX_STEM: usize = 80;

/// The highest number appended to a taken name (a folder never holds this
/// many clips of one name; past it the save fails on the unique path).
const MAX_SUFFIX: u32 = 100_000;

/// The stem of an unnamed or unprintable clip.
const FALLBACK_STEM: &str = "Clip";

/// `name` as a file stem: path separators, control and reserved characters
/// become spaces, runs of spaces collapse, and leading dots go (no hidden
/// files, no `..`). Never empty.
pub fn safe_stem(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|character| if allowed(character) { character } else { ' ' })
        .collect();
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    let joined = words.join(" ");
    let trimmed = joined.trim_start_matches(['.', ' ']);
    let stem: String = trimmed.chars().take(MAX_STEM).collect();
    let stem = stem.trim_end().to_owned();
    if stem.is_empty() {
        FALLBACK_STEM.to_owned()
    } else {
        stem
    }
}

/// Whether `character` may stay in a file name on macOS, Windows and Linux.
fn allowed(character: char) -> bool {
    !character.is_control()
        && !matches!(
            character,
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
        )
}

/// `stem.extension`, or `stem 2.extension`, `stem 3.extension`, … : the
/// first file name not in `taken` (compared case-insensitively, as APFS does
/// by default).
pub fn unique_file_name<S: BuildHasher>(
    stem: &str,
    extension: &str,
    taken: &HashSet<String, S>,
) -> String {
    let free = |candidate: &str| !taken.contains(&candidate.to_lowercase());
    let first = format!("{stem}.{extension}");
    if free(&first) {
        return first;
    }
    (2..=MAX_SUFFIX)
        .map(|n| format!("{stem} {n}.{extension}"))
        .find(|candidate| free(candidate))
        .unwrap_or(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stem_keeps_the_name_but_never_a_path_or_a_reserved_character() {
        assert_eq!(
            safe_stem("Pricing: the \"big\" call"),
            "Pricing the big call"
        );
        assert_eq!(safe_stem("../../etc/passwd"), "etc passwd");
        assert_eq!(safe_stem("a\\b\u{0}c\nd"), "a b c d");
        assert_eq!(safe_stem("  .hidden  "), "hidden");
        assert_eq!(safe_stem("Café ☕ décision"), "Café ☕ décision");
    }

    #[test]
    fn an_empty_or_unprintable_name_falls_back() {
        assert_eq!(safe_stem(""), "Clip");
        assert_eq!(safe_stem("///"), "Clip");
        assert_eq!(safe_stem("..."), "Clip");
    }

    #[test]
    fn a_long_name_is_cut_on_a_character_boundary() {
        let stem = safe_stem(&"é".repeat(200));
        assert_eq!(stem.chars().count(), MAX_STEM);
    }

    #[test]
    fn a_taken_name_gets_the_next_free_number_ignoring_case() {
        let taken: HashSet<String> = ["intro.m4a", "intro 2.m4a"].map(String::from).into();

        assert_eq!(unique_file_name("Outro", "m4a", &taken), "Outro.m4a");
        assert_eq!(unique_file_name("INTRO", "m4a", &taken), "INTRO 3.m4a");
        assert_eq!(unique_file_name("intro", "mp4", &taken), "intro.mp4");
    }
}

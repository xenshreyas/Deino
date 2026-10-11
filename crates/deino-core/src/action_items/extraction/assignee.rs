//! Assignees from names: [`Roster::resolve`] maps the name the model wrote
//! ("Maya", "maya chen", "Jordan Elis", "me", "Speaker 2") to a person.
//!
//! The meeting's attendees are searched first, then the people its speakers
//! were linked to (CAP-04), so an invitee wins over a namesake who only
//! spoke. Within a group the rules go from strict to loose: the whole name;
//! every word of what was said in a person's name ("Maya" in "Maya Chen");
//! the address before the `@`; then one or two typos (transcribed names are
//! often misspelt). A rule that matches two people is ambiguous and leaves
//! the item unassigned rather than guessing.

use deino_domain::PersonId;

/// What the model writes when nobody owns an item.
const NOBODY: [&str; 9] = [
    "",
    "none",
    "null",
    "nobody",
    "unassigned",
    "unknown",
    "team",
    "everyone",
    "we",
];
/// What the model writes for the user.
const ME: [&str; 5] = ["me", "i", "myself", "the user", "user"];

/// A person an item can be assigned to, as the meeting knows them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// The person.
    pub person: PersonId,
    /// A name they go by here: their name, or a speaker label.
    pub name: String,
    /// Their address, if known.
    pub email: Option<String>,
}

/// The people of one meeting, in the order names are matched against them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Roster {
    /// The user's person, for "me".
    pub me: Option<PersonId>,
    /// The meeting's attendees (searched first).
    pub attendees: Vec<Candidate>,
    /// The people its speakers are linked to, by name and by speaker label.
    pub speakers: Vec<Candidate>,
}

impl Roster {
    /// The person `said` names, if exactly one fits.
    pub fn resolve(&self, said: &str) -> Option<PersonId> {
        let said = normalize(said);
        if NOBODY.contains(&said.as_str()) {
            return None;
        }
        if ME.contains(&said.as_str()) {
            return self.me;
        }
        for group in [&self.attendees, &self.speakers] {
            match match_in(group, &said) {
                Match::One(person) => return Some(person),
                // Two people fit: guessing would assign it to the wrong one.
                Match::Ambiguous => return None,
                Match::None => {}
            }
        }
        None
    }
}

/// What a name matched in one group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Match {
    /// Exactly one person.
    One(PersonId),
    /// More than one person.
    Ambiguous,
    /// Nobody.
    None,
}

/// The one person in `group` that `said` names, by the strictest rule that
/// matches anyone.
fn match_in(group: &[Candidate], said: &str) -> Match {
    let rules: [&dyn Fn(&Candidate) -> bool; 3] = [
        &|candidate| normalize(&candidate.name) == said,
        &|candidate| words_within(said, &normalize(&candidate.name)),
        &|candidate| email_names(candidate).iter().any(|name| name == said),
    ];
    for rule in rules {
        let mut matches: Vec<PersonId> = group
            .iter()
            .filter(|candidate| rule(candidate))
            .map(|candidate| candidate.person)
            .collect();
        matches.sort_unstable();
        matches.dedup();
        match matches.as_slice() {
            [one] => return Match::One(*one),
            [] => {}
            _ => return Match::Ambiguous,
        }
    }
    closest(group, said)
}

/// The one person whose name, or first name, is within a typo or two of `said`.
fn closest(group: &[Candidate], said: &str) -> Match {
    // "Speaker 4" is not a typo of "Speaker 3".
    if said.chars().any(|character| character.is_ascii_digit()) {
        return Match::None;
    }
    let allowed = if said.chars().count() >= 8 { 2 } else { 1 };
    let mut best: Option<(usize, PersonId)> = None;
    let mut tied = false;
    for candidate in group {
        let name = normalize(&candidate.name);
        let first = name.split(' ').next().unwrap_or_default().to_owned();
        let distance = edit_distance(said, &name).min(edit_distance(said, &first));
        match best {
            Some((shortest, person)) if distance == shortest && person != candidate.person => {
                tied = true;
            }
            Some((shortest, _)) if distance >= shortest => {}
            _ => {
                best = Some((distance, candidate.person));
                tied = false;
            }
        }
    }
    match best {
        Some((distance, _)) if distance <= allowed && tied => Match::Ambiguous,
        Some((distance, person)) if distance <= allowed => Match::One(person),
        _ => Match::None,
    }
}

/// True when every word of `said` is a word of `name`.
fn words_within(said: &str, name: &str) -> bool {
    let words: Vec<&str> = name.split(' ').collect();
    !said.is_empty() && said.split(' ').all(|word| words.contains(&word))
}

/// The names an address spells before its `@`: "marcus.webb" gives
/// "marcus webb" and "marcus".
fn email_names(candidate: &Candidate) -> Vec<String> {
    let Some((local, _)) = candidate
        .email
        .as_deref()
        .and_then(|email| email.split_once('@'))
    else {
        return Vec::new();
    };
    let spaced = normalize(&local.replace(['.', '_', '-'], " "));
    let first = spaced.split(' ').next().unwrap_or_default().to_owned();
    vec![spaced, first]
}

/// Lower case, accents folded, punctuation dropped, single spaces; a
/// trailing "(me)" marker removed.
pub fn normalize(name: &str) -> String {
    let lower = name.trim().to_lowercase();
    let folded: String = lower
        .trim_end_matches("(me)")
        .chars()
        .map(fold_accent)
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect();
    folded.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The unaccented letter for common Latin accented letters.
fn fold_accent(character: char) -> char {
    match character {
        'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ø' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ñ' => 'n',
        'ç' => 'c',
        other => other,
    }
}

/// Levenshtein distance between `a` and `b`, by characters.
pub fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (row, left) in a.chars().enumerate() {
        let mut current = Vec::with_capacity(b.len() + 1);
        current.push(row + 1);
        for (column, right) in b.iter().enumerate() {
            let substitute =
                previous.get(column).copied().unwrap_or_default() + usize::from(left != *right);
            let delete = previous.get(column + 1).copied().unwrap_or_default() + 1;
            let insert = current.last().copied().unwrap_or_default() + 1;
            current.push(substitute.min(delete).min(insert));
        }
        previous = current;
    }
    previous.last().copied().unwrap_or_default()
}

#[cfg(test)]
#[path = "assignee_tests.rs"]
mod tests;

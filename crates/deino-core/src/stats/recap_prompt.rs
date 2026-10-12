//! The `recap` prompt: pure, so tests check it without a model.
//!
//! The year in review reads aggregated numbers and meeting titles with
//! their dates, never transcripts or notes (ORG-09): the recap stays a
//! light read of the year, and the prompt stays small enough for any model.

use std::cmp::Reverse;

use deino_domain::LocalZone;
use deino_llm::Prompt;
use deino_llm::prompts::{PromptText, RecapSystem, RecapUser};

use super::repo::TitleFact;
use super::{StatsOverview, TopPerson};

/// Meeting titles read from the year, at most.
pub const TITLES_READ: u32 = 5_000;
/// Meeting titles in the prompt, at most: a busier year is sampled evenly
/// across its months rather than cut short.
const MAX_TITLES: usize = 400;
/// Characters of one title in the prompt.
const TITLE_CHARS: usize = 120;
/// Room for the answer: under 250 words.
const ANSWER_TOKENS: u32 = 900;
/// Seconds in an hour.
const HOUR: u64 = 3_600;
/// Weekday names, Monday first.
const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// The `recap` prompt for `year`.
pub fn recap_prompt(
    year: i32,
    stats: &StatsOverview,
    titles: &[TitleFact],
    zone: &dyn LocalZone,
) -> Prompt {
    let year = year.to_string();
    let numbers = number_lines(stats);
    let step = titles.len().div_ceil(MAX_TITLES).max(1);
    let titles: Vec<String> = titles
        .iter()
        .step_by(step)
        .map(|title| title_line(title, zone))
        .collect();
    let user = RecapUser {
        year: &year,
        numbers: &numbers,
        titles: &titles,
    };
    Prompt::new(user.text())
        .system(RecapSystem.text())
        .max_tokens(ANSWER_TOKENS)
}

/// `Label: value` lines for the numbers of the year.
pub fn number_lines(stats: &StatsOverview) -> Vec<String> {
    let audience = stats.audience;
    let mut lines = vec![
        format!("Meetings: {}", stats.meetings),
        format!(
            "Hours recorded: {}",
            (stats.recorded_seconds + HOUR / 2) / HOUR
        ),
        format!("Different people met: {}", stats.people),
        format!(
            "Meetings with only colleagues: {}; with outside people: {}",
            audience.internal, audience.external
        ),
    ];
    if !stats.top_people.is_empty() {
        lines.push(format!("Met most: {}", people_line(&stats.top_people)));
    }
    lines.extend(busiest("Busiest weekday", &stats.weekdays, |slot| {
        WEEKDAYS.get(slot).map(|day| (*day).to_owned())
    }));
    lines.extend(busiest("Busiest hour", &stats.hours, |slot| {
        Some(format!("{slot:02}:00"))
    }));
    lines
}

fn people_line(people: &[TopPerson]) -> String {
    people
        .iter()
        .map(|person| format!("{} ({})", person.name, person.meetings))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `label: name (count)` for the fullest slot, if any slot has a meeting.
fn busiest(label: &str, counts: &[u32], name: impl Fn(usize) -> Option<String>) -> Option<String> {
    let (slot, most) = counts
        .iter()
        .copied()
        .enumerate()
        .max_by_key(|&(slot, count)| (count, Reverse(slot)))?;
    if most == 0 {
        return None;
    }
    Some(format!("{label}: {} ({most} meetings)", name(slot)?))
}

fn title_line(title: &TitleFact, zone: &dyn LocalZone) -> String {
    let day = zone.day_of(title.at).format("%Y-%m-%d");
    let text: String = title.title.trim().chars().take(TITLE_CHARS).collect();
    format!("{day} {text}")
}

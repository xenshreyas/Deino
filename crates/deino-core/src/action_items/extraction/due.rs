//! Due dates from what was said: [`resolve_due`] turns the model's
//! `due_phrase` ("by Friday", "end of day tomorrow", "the ninth of October")
//! into a calendar day, counted from the meeting's local day.
//!
//! The phrase is read here, deterministically, rather than trusted to the
//! model's own date arithmetic, which slips across week and month ends. The
//! model's `due` is the fallback for wording these rules do not cover, and
//! only when it is plausible (not before the meeting, not years after).
//!
//! Conventions (each pinned by a test): weeks run Monday to Sunday; a bare
//! weekday is its next occurrence after the meeting day; "next Friday" and
//! "Friday next week" are the Friday of the following week; "this week" and
//! "end of the week" are that week's Friday; a day of the month without a
//! month ("the ninth") is its next occurrence on or after the meeting day.

use chrono::{Datelike, Days, NaiveDate, Weekday};

use super::words::{Words, month_named, number_word, ordinal_day};
use crate::action_items::next_monday;

/// The latest plausible due day, in days after the meeting.
const MAX_DAYS_AHEAD: u64 = 400;

/// The due day for `phrase` (the words said) or, failing that, the model's
/// own `model_date` (`YYYY-MM-DD`), counted from `meeting_day`; `None` when
/// neither gives a plausible day.
pub fn resolve_due(
    phrase: Option<&str>,
    model_date: Option<&str>,
    meeting_day: NaiveDate,
) -> Option<NaiveDate> {
    phrase
        .and_then(|phrase| parse_phrase(phrase, meeting_day))
        .or_else(|| model_date.and_then(|date| date.trim().parse().ok()))
        .filter(|day| plausible(*day, meeting_day))
}

/// A due day on or after the meeting and not absurdly far ahead.
fn plausible(day: NaiveDate, meeting_day: NaiveDate) -> bool {
    day >= meeting_day && day <= meeting_day + Days::new(MAX_DAYS_AHEAD)
}

/// The rules, most specific first; the first that recognises the phrase wins.
const RULES: [fn(&Words, NaiveDate) -> Option<NaiveDate>; 8] = [
    month_and_day,
    day_of_month,
    tomorrow,
    in_some_days,
    end_of_month,
    weekday,
    week,
    today,
];

/// The day `phrase` names, counted from `meeting_day`.
pub fn parse_phrase(phrase: &str, meeting_day: NaiveDate) -> Option<NaiveDate> {
    let words = Words::of(phrase);
    RULES.iter().find_map(|rule| rule(&words, meeting_day))
}

/// "October 9", "9 October", "the ninth of October", "Oct 9th, 2026".
fn month_and_day(words: &Words, from: NaiveDate) -> Option<NaiveDate> {
    let (index, month) = words.find_map(month_named)?;
    let day = [index + 1, index.wrapping_sub(1), index.wrapping_sub(2)]
        .into_iter()
        .find_map(|at| words.get(at).and_then(day_number))?;
    let year = words.find_map(year_number).map(|(_, year)| year);
    match year {
        Some(year) => NaiveDate::from_ymd_opt(year, month, day),
        None => next_date(month, day, from),
    }
}

/// "the ninth", "Friday the 13th": the next such day of the month.
fn day_of_month(words: &Words, from: NaiveDate) -> Option<NaiveDate> {
    let (_, day) = words.find_map(ordinal_day)?;
    let this_month =
        NaiveDate::from_ymd_opt(from.year(), from.month(), day).filter(|date| *date >= from);
    this_month.or_else(|| {
        let next = from.checked_add_months(chrono::Months::new(1))?;
        NaiveDate::from_ymd_opt(next.year(), next.month(), day)
    })
}

/// "tomorrow", "end of day tomorrow"; "the day after tomorrow".
fn tomorrow(words: &Words, from: NaiveDate) -> Option<NaiveDate> {
    if !words.has("tomorrow") {
        return None;
    }
    let days = if words.has_run(&["day", "after", "tomorrow"]) {
        2
    } else {
        1
    };
    from.checked_add_days(Days::new(days))
}

/// "in three days", "in 2 weeks", "within a week".
fn in_some_days(words: &Words, from: NaiveDate) -> Option<NaiveDate> {
    let (unit_at, per_unit) = words.find_map(|word| match word {
        "day" | "days" => Some(1),
        "week" | "weeks" => Some(7),
        _ => None,
    })?;
    let count_word = words.get(unit_at.checked_sub(1)?)?;
    if !words.has("in") && !words.has("within") {
        return None;
    }
    let count = number_word(count_word)?;
    from.checked_add_days(Days::new(count * per_unit))
}

/// "end of the month", "end of October", "the end of next month".
fn end_of_month(words: &Words, from: NaiveDate) -> Option<NaiveDate> {
    let (end, ()) = words.find_map(|word| (word == "end").then_some(()))?;
    let named = |offset: usize| words.get(end + offset);
    let month = match (named(2), named(3)) {
        (Some("month"), _) | (Some("the"), Some("month")) => from.month(),
        (Some("next"), Some("month")) => from.checked_add_months(chrono::Months::new(1))?.month(),
        (Some(word), _) => month_named(word)?,
        _ => return None,
    };
    let first = next_date(
        month,
        1,
        NaiveDate::from_ymd_opt(from.year(), from.month(), 1)?,
    )?;
    first.checked_add_months(chrono::Months::new(1))?.pred_opt()
}

/// "Friday", "this Friday", "next Friday", "Friday next week".
fn weekday(words: &Words, from: NaiveDate) -> Option<NaiveDate> {
    let (index, day) = words.find_map(weekday_named)?;
    let following_week =
        words.get(index.wrapping_sub(1)) == Some("next") || words.has_run(&["next", "week"]);
    if following_week {
        let offset = u64::from(day.num_days_from_monday());
        return next_monday(from).checked_add_days(Days::new(offset));
    }
    let ahead = (7 + day.num_days_from_monday() - from.weekday().num_days_from_monday()) % 7;
    let this_day = words.get(index.wrapping_sub(1)) == Some("this") && ahead == 0;
    let ahead = if ahead == 0 && !this_day { 7 } else { ahead };
    from.checked_add_days(Days::new(u64::from(ahead)))
}

/// "this week", "end of the week" (its Friday); "next week" (the next one's).
fn week(words: &Words, from: NaiveDate) -> Option<NaiveDate> {
    if !words.has("week") && !words.has("eow") {
        return None;
    }
    let monday = if words.has("next") {
        next_monday(from)
    } else {
        from - Days::new(u64::from(from.weekday().num_days_from_monday()))
    };
    let friday = monday.checked_add_days(Days::new(4))?;
    Some(friday.max(from))
}

/// "today", "tonight", "this afternoon", "end of day", "after this call".
fn today(words: &Words, from: NaiveDate) -> Option<NaiveDate> {
    let said = ["today", "tonight", "eod", "afternoon", "morning", "evening"]
        .iter()
        .any(|word| words.has(word))
        || words.has_run(&["end", "of", "day"])
        || words.has_run(&["end", "of", "the", "day"])
        || words.has_run(&["after", "this"])
        || words.has_run(&["right", "after"]);
    said.then_some(from)
}

/// `month`/`day` on or after `from` (this year, else next year).
fn next_date(month: u32, day: u32, from: NaiveDate) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(from.year(), month, day)
        .filter(|date| *date >= from)
        .or_else(|| NaiveDate::from_ymd_opt(from.year() + 1, month, day))
}

/// A day of the month written as digits ("9", "9th") or an ordinal word.
fn day_number(word: &str) -> Option<u32> {
    ordinal_day(word).or_else(|| word.parse().ok().filter(|day| (1..=31).contains(day)))
}

/// A four-digit year.
fn year_number(word: &str) -> Option<i32> {
    word.parse()
        .ok()
        .filter(|year| (2000..=2100).contains(year))
}

/// The weekday a word names ("fri", "friday", "fridays").
fn weekday_named(word: &str) -> Option<Weekday> {
    let word = word.trim_end_matches('s');
    [
        ("mon", Weekday::Mon),
        ("tue", Weekday::Tue),
        ("wed", Weekday::Wed),
        ("thu", Weekday::Thu),
        ("fri", Weekday::Fri),
        ("sat", Weekday::Sat),
        ("sun", Weekday::Sun),
    ]
    .into_iter()
    .find(|(prefix, _)| word.len() >= 3 && word.starts_with(prefix) && full_day(word, prefix))
    .map(|(_, day)| day)
}

/// `word` is the abbreviation or a spelling of the day starting `prefix`.
fn full_day(word: &str, prefix: &str) -> bool {
    word == prefix || word.ends_with("day") || matches!(word, "tues" | "thur" | "thurs" | "weds")
}

#[cfg(test)]
#[path = "due_tests.rs"]
mod tests;

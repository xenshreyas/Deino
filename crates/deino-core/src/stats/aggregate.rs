//! Pure aggregation: the repository's rows in, the page's numbers out.
//!
//! Local days and hours come from the [`LocalZone`] port, so every bucket
//! is the user's wall clock and tests pin the zone. Who counts as "the
//! user" and who is internal come from `profile::InternalDomains`.

use std::collections::{BTreeMap, HashMap};

use chrono::{Datelike, NaiveDate, Timelike};
use deino_domain::{Audience, Email, LocalZone, MeetingId, PersonId};

use super::pricing::price_of;
use super::repo::{AttendanceFact, MeetingFact, UsageFact};
use super::{AudienceSplit, HeatmapDay, LlmUsage, ModelUsage, TopPerson};
use crate::profile::InternalDomains;

/// People listed under "Top people".
pub const TOP_PEOPLE: usize = 8;

/// `n` as a count for the UI, saturating (no period has 4 billion meetings).
pub(super) fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The recorded seconds of `meetings`; unknown lengths count as zero.
pub fn recorded_seconds(meetings: &[MeetingFact]) -> u64 {
    meetings
        .iter()
        .filter_map(|meeting| meeting.duration_s)
        .map(|seconds| u64::try_from(seconds).unwrap_or(0))
        .sum()
}

/// Meetings per local weekday, Monday first.
pub fn by_weekday(meetings: &[MeetingFact], zone: &dyn LocalZone) -> Vec<u32> {
    tally(meetings, 7, |meeting| {
        zone.local(meeting.at).weekday().num_days_from_monday()
    })
}

/// Meetings per local starting hour, midnight first.
pub fn by_hour(meetings: &[MeetingFact], zone: &dyn LocalZone) -> Vec<u32> {
    tally(meetings, 24, |meeting| zone.local(meeting.at).hour())
}

fn tally(meetings: &[MeetingFact], slots: usize, slot: impl Fn(&MeetingFact) -> u32) -> Vec<u32> {
    let mut counts = vec![0_u32; slots];
    for meeting in meetings {
        let index = usize::try_from(slot(meeting)).unwrap_or(usize::MAX);
        if let Some(count) = counts.get_mut(index) {
            *count += 1;
        }
    }
    counts
}

/// Meetings per local day within `[first, last]`, days without any left out.
pub fn per_day(
    meetings: &[MeetingFact],
    zone: &dyn LocalZone,
    (first, last): (NaiveDate, NaiveDate),
) -> Vec<HeatmapDay> {
    let mut days: BTreeMap<NaiveDate, u32> = BTreeMap::new();
    for meeting in meetings {
        let day = zone.day_of(meeting.at);
        if (first..=last).contains(&day) {
            *days.entry(day).or_default() += 1;
        }
    }
    days.into_iter()
        .map(|(day, meetings)| HeatmapDay { day, meetings })
        .collect()
}

/// True when `fact` is the user: their own person, or one of their addresses.
fn is_user(fact: &AttendanceFact, domains: &InternalDomains) -> bool {
    fact.is_me
        || fact
            .email
            .as_ref()
            .is_some_and(|email| domains.is_own(email))
}

/// How many different people the user met, and who they met most: most
/// meetings first, then by name.
pub fn people(attendance: &[AttendanceFact], domains: &InternalDomains) -> (u32, Vec<TopPerson>) {
    let mut met: HashMap<&PersonId, (&str, u32)> = HashMap::new();
    for fact in attendance.iter().filter(|fact| !is_user(fact, domains)) {
        met.entry(&fact.person_id).or_insert((&fact.name, 0)).1 += 1;
    }
    let unique = count(met.len());
    let mut ranked: Vec<TopPerson> = met
        .into_iter()
        .map(|(id, (name, meetings))| TopPerson {
            id: id.clone(),
            name: name.to_owned(),
            meetings,
        })
        .collect();
    ranked.sort_by(|a, b| {
        (b.meetings, a.name.to_lowercase(), &a.id).cmp(&(a.meetings, b.name.to_lowercase(), &b.id))
    });
    ranked.truncate(TOP_PEOPLE);
    (unique, ranked)
}

/// The meetings by who they were with. A meeting with no attendee address,
/// or a profile with no address of the user's, is unknown.
pub fn audience(
    meetings: &[MeetingFact],
    attendance: &[AttendanceFact],
    domains: &InternalDomains,
) -> AudienceSplit {
    let mut emails: HashMap<&MeetingId, Vec<&Email>> = HashMap::new();
    for fact in attendance {
        if let Some(email) = &fact.email {
            emails.entry(&fact.meeting_id).or_default().push(email);
        }
    }
    let mut split = AudienceSplit::default();
    for meeting in meetings {
        let attendees = emails
            .get(&meeting.id)
            .map(Vec::as_slice)
            .unwrap_or_default();
        match domains.classify(attendees.iter().copied()) {
            Some(Audience::Internal) => split.internal += 1,
            Some(Audience::External) => split.external += 1,
            None => split.unknown += 1,
        }
    }
    split
}

/// LLM usage with its estimated cost.
pub fn llm_usage(facts: Vec<UsageFact>) -> LlmUsage {
    let models: Vec<ModelUsage> = facts.into_iter().map(model_usage).collect();
    let estimated_cost_micros = models
        .iter()
        .filter_map(|model| model.estimated_cost_micros)
        .sum();
    let unpriced_calls = models
        .iter()
        .filter(|model| model.estimated_cost_micros.is_none())
        .map(|model| model.calls)
        .sum();
    LlmUsage {
        models,
        estimated_cost_micros,
        unpriced_calls,
    }
}

fn model_usage(fact: UsageFact) -> ModelUsage {
    let input_tokens = u64::try_from(fact.input_tokens).unwrap_or(0);
    let output_tokens = u64::try_from(fact.output_tokens).unwrap_or(0);
    let estimated_cost_micros = price_of(fact.provider, &fact.model)
        .map(|price| price.cost_micros(input_tokens, output_tokens));
    ModelUsage {
        provider: fact.provider,
        model: fact.model,
        calls: fact.calls,
        failed: fact.failed,
        input_tokens,
        output_tokens,
        estimated_cost_micros,
    }
}

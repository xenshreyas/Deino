//! Grouping ranked hits into the sections the UI shows.

use std::collections::HashSet;

use deino_domain::MeetingId;

use super::{Hit, HitKind, SearchFilters, SearchResults};

/// `hits` (best first) as sections of at most `limit` each. A meeting is in
/// the Meetings section once, with its best hit: its title, notes, summary
/// or a moment of its transcript.
pub fn group(hits: Vec<Hit>, limit: usize, filters: SearchFilters) -> SearchResults {
    let mut results = SearchResults {
        filters,
        ..SearchResults::default()
    };
    let mut seen: HashSet<MeetingId> = HashSet::new();
    for hit in hits {
        if hit.kind.is_meeting_content()
            && let Some(meeting) = &hit.meeting_id
            && seen.insert(meeting.clone())
        {
            push(&mut results.meetings, hit.clone(), limit);
        }
        let section = match hit.kind {
            HitKind::Segment => &mut results.moments,
            HitKind::ActionItem => &mut results.action_items,
            HitKind::Person => &mut results.people,
            HitKind::Company => &mut results.companies,
            HitKind::Folder => &mut results.folders,
            HitKind::Meeting | HitKind::Note | HitKind::Summary => continue,
        };
        push(section, hit, limit);
    }
    results
}

fn push(section: &mut Vec<Hit>, hit: Hit, limit: usize) {
    if section.len() < limit {
        section.push(hit);
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::Millis;

    use super::*;
    use crate::search::Snippet;

    fn hit(kind: HitKind, ref_id: &str, meeting: Option<&str>) -> Hit {
        Hit {
            kind,
            ref_id: ref_id.to_owned(),
            meeting_id: meeting.map(MeetingId::new),
            ts_ms: (kind == HitKind::Segment).then_some(Millis(1_000)),
            end_ms: None,
            label: Snippet::plain(ref_id),
            snippet: Snippet::default(),
            date: None,
            score: 1.0,
        }
    }

    fn refs(hits: &[Hit]) -> Vec<&str> {
        hits.iter().map(|hit| hit.ref_id.as_str()).collect()
    }

    #[test]
    fn a_meeting_appears_once_with_its_best_hit_and_moments_stay_moments() {
        let hits = vec![
            hit(HitKind::Segment, "s1", Some("m1")),
            hit(HitKind::Meeting, "m1", Some("m1")),
            hit(HitKind::Note, "m2", Some("m2")),
            hit(HitKind::Segment, "s2", Some("m2")),
        ];

        let results = group(hits, 10, SearchFilters::default());

        assert_eq!(refs(&results.meetings), ["s1", "m2"]);
        assert_eq!(refs(&results.moments), ["s1", "s2"]);
    }

    #[test]
    fn entities_go_to_their_own_sections_and_each_section_is_capped() {
        let hits = vec![
            hit(HitKind::Person, "maya", None),
            hit(HitKind::Person, "tom", None),
            hit(HitKind::Company, "acme", None),
            hit(HitKind::Folder, "q4", None),
            hit(HitKind::ActionItem, "send deck", Some("m1")),
        ];

        let results = group(hits, 1, SearchFilters::default());

        assert_eq!(refs(&results.people), ["maya"]);
        assert_eq!(refs(&results.companies), ["acme"]);
        assert_eq!(refs(&results.folders), ["q4"]);
        assert_eq!(refs(&results.action_items), ["send deck"]);
        assert!(
            results.meetings.is_empty(),
            "action items are not meeting content"
        );
    }
}

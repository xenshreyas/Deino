//! Search results as Markdown.

use deino_core::query::link::clock_time;
use deino_core::query::{HitKind, SearchHit, SearchMode};
use deino_domain::Millis;

use super::{day, line};
use crate::request::{Markdown, SearchResults};

impl Markdown for SearchResults {
    fn markdown(&self) -> String {
        let mut out = String::new();
        let mode = mode(self.mode);
        if self.hits.is_empty() {
            line(
                &mut out,
                format!("Nothing matches \"{}\" ({mode}).", self.query),
            );
            return out;
        }
        line(
            &mut out,
            format!(
                "# Search \"{}\" ({} hits, {mode})\n",
                self.query,
                self.hits.len()
            ),
        );
        for hit in &self.hits {
            line(&mut out, format!("- {}", heading(hit)));
            let snippet: Vec<&str> = hit.snippet.split_whitespace().collect();
            line(&mut out, format!("  > {}", snippet.join(" ")));
        }
        out
    }
}

fn mode(mode: SearchMode) -> &'static str {
    match mode {
        SearchMode::Keyword => "keyword search",
        SearchMode::Hybrid => "keyword and semantic search",
    }
}

fn heading(hit: &SearchHit) -> String {
    let mut parts = vec![kind(hit.kind).to_owned(), format!("**{}**", hit.label)];
    if let Some(held) = &hit.held_at {
        parts.push(day(held.date_naive()));
    }
    if let Some(at) = hit.at_ms {
        parts.push(format!("at {}", clock_time(Millis(at))));
    }
    if let Some(id) = &hit.meeting_id {
        parts.push(format!("`{id}`"));
    }
    if let Some(link) = &hit.link {
        parts.push(link.clone());
    }
    parts.join(" · ")
}

fn kind(kind: HitKind) -> &'static str {
    match kind {
        HitKind::Meeting => "title",
        HitKind::Notes => "my notes",
        HitKind::AiNotes => "AI notes",
        HitKind::Transcript => "transcript",
        HitKind::ActionItem => "action item",
        HitKind::Person => "person",
        HitKind::Company => "company",
        HitKind::Folder => "folder",
    }
}

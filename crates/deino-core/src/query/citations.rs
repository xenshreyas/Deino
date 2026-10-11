//! The transcript moments a notes document cites, as `deino://` links.
//!
//! Readable Markdown writes a citation as `[12:04]`; [`citations`] lists
//! each cited moment once, with that label and the link to it, so a reader
//! of the Markdown can follow every `[12:04]` to the recording.

use deino_doc::{Block, Doc, Inline};
use deino_domain::{MeetingId, Millis};

use super::CitationLink;
use super::link::{clock_time, moment_link};

/// Every moment `doc` cites, once each, in time order.
pub(super) fn citations(doc: &Doc, meeting: &MeetingId) -> Vec<CitationLink> {
    let mut moments = Vec::new();
    for block in &doc.content {
        collect(block, &mut moments);
    }
    moments.sort_unstable();
    moments.dedup();
    moments
        .into_iter()
        .map(|at| CitationLink {
            label: clock_time(at),
            at_ms: at.get(),
            link: moment_link(meeting, at),
        })
        .collect()
}

/// The moments cited in `block` and the blocks inside it.
fn collect(block: &Block, moments: &mut Vec<Millis>) {
    for inline in block.inline_content() {
        if let Inline::Citation { attrs, .. } = inline {
            moments.push(attrs.start_ms);
        }
    }
    for child in block.children() {
        collect(child, moments);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_citations_are_listed_once_in_time_order() {
        let markdown = "## Decisions\n\n\
            - Ship it <citation data-segment-id=\"0199f000-0000-7000-8000-000000000002\" data-start-ms=\"724000\" />\n  \
            - Later <citation data-segment-id=\"0199f000-0000-7000-8000-000000000001\" data-start-ms=\"5000\" />\n\
            - Again <citation data-segment-id=\"0199f000-0000-7000-8000-000000000002\" data-start-ms=\"724000\" />\n";
        let doc = deino_doc::from_markdown(markdown).unwrap();

        let links = citations(&doc, &MeetingId::new("meeting-1"));

        let labels: Vec<&str> = links.iter().map(|link| link.label.as_str()).collect();
        assert_eq!(labels, ["0:05", "12:04"]);
        assert_eq!(links[1].link, "deino://meeting/meeting-1?t=724000");
    }
}

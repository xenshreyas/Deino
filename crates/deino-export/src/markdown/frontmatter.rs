//! YAML frontmatter that Obsidian reads as properties.
//!
//! Strings are written as JSON strings, which are valid YAML double-quoted
//! scalars, so no title can break the block. `deino-id` marks a file as
//! Deino's: auto-export only ever overwrites files carrying its own ID.

use super::links;
use crate::{LinkStyle, MeetingExport, labels};

/// The key that names the meeting a file was exported from.
pub(crate) const OWNER_KEY: &str = "deino-id";

/// A frontmatter block being written, one property at a time.
#[derive(Default)]
pub(crate) struct Frontmatter {
    lines: Vec<String>,
}

impl Frontmatter {
    /// A text property.
    pub(crate) fn text(mut self, key: &str, value: &str) -> Self {
        self.lines.push(format!("{key}: {}", quoted(value)));
        self
    }

    /// A value written as is (a date or a number).
    pub(crate) fn raw(mut self, key: &str, value: &str) -> Self {
        self.lines.push(format!("{key}: {value}"));
        self
    }

    /// A list property; `[]` when empty.
    pub(crate) fn list(mut self, key: &str, values: &[String]) -> Self {
        if values.is_empty() {
            self.lines.push(format!("{key}: []"));
        } else {
            self.lines.push(format!("{key}:"));
            self.lines
                .extend(values.iter().map(|value| format!("  - {}", quoted(value))));
        }
        self
    }

    /// The block between its `---` fences.
    pub(crate) fn build(self) -> String {
        format!("---\n{}\n---", self.lines.join("\n"))
    }
}

/// The meeting note's properties: title, date, duration, attendees, tags,
/// folders, the `deino://` link and the owner ID.
pub(crate) fn meeting(export: &MeetingExport, style: LinkStyle) -> Frontmatter {
    let attendees: Vec<String> = export
        .attendees
        .iter()
        .map(|person| links::person_property(&person.name, style))
        .collect();
    let tags = match style {
        LinkStyle::Plain => export.tags.clone(),
        LinkStyle::Obsidian => links::tag_slugs(&export.tags),
    };
    let mut frontmatter = Frontmatter::default()
        .text("title", &export.title)
        .raw("date", &labels::iso_local(&export.started_at));
    if let Some(duration) = export.duration {
        frontmatter = frontmatter.text("duration", &labels::duration(duration));
    }
    frontmatter
        .list("attendees", &attendees)
        .list("tags", &tags)
        .list("folders", &export.folders)
        .text("link", &export.link())
        .text(OWNER_KEY, export.id.as_str())
}

/// The meeting ID in `head`'s frontmatter, if it was exported by Deino.
pub fn owner(head: &str) -> Option<String> {
    let block = head.strip_prefix("---\n")?;
    let end = block.find("\n---")?;
    block.get(..end)?.lines().find_map(|line| {
        let value = line.strip_prefix(OWNER_KEY)?.strip_prefix(':')?.trim();
        serde_json::from_str::<String>(value).ok()
    })
}

fn quoted(value: &str) -> String {
    // A `String` always serializes; the fallback only satisfies the type.
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_are_quoted_so_no_title_breaks_the_block() {
        let block = Frontmatter::default()
            .text("title", "Q4: \"plan\"\n---")
            .list("tags", &[])
            .build();
        assert_eq!(
            block,
            "---\ntitle: \"Q4: \\\"plan\\\"\\n---\"\ntags: []\n---"
        );
    }

    #[test]
    fn the_owner_is_read_back_from_a_files_head() {
        let head = "---\ntitle: \"x\"\ndeino-id: \"meeting-1\"\n---\n\n# x\n";
        assert_eq!(owner(head).as_deref(), Some("meeting-1"));
        assert_eq!(owner("# no frontmatter\ndeino-id: \"meeting-1\""), None);
        assert_eq!(owner("---\ntitle: \"x\"\n---\ndeino-id: \"m\""), None);
    }
}

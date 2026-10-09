//! The inline HTML that carries what Markdown cannot say, both directions.
//!
//! [`Flavor::Lossless`](crate::Flavor::Lossless) Markdown writes these tags
//! and [`from_markdown`](crate::from_markdown) reads them back, so the pair
//! lives in one place:
//!
//! | Model | Markdown |
//! |---|---|
//! | [`Mark::Underline`] | `<u>…</u>` |
//! | [`Mark::Origin`] | `<span data-origin="ai" data-edited="true">…</span>` |
//! | [`Inline::Mention`] | `<mention data-id="…" data-label="…" />` |
//! | [`Inline::Citation`] | `<citation data-segment-id="…" data-start-ms="…" />` |
//!
//! Attribute values are entity-escaped, so a label may hold quotes or `<`.

use deino_domain::{Millis, PersonId, SegmentId};

use crate::{Author, CitationAttrs, Inline, Mark, MentionAttrs, OriginAttrs};

/// What one inline HTML tag means to the document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Tag {
    /// Starts a mark that lasts until its closing tag.
    Open(Mark),
    /// Ends the innermost open mark of this kind.
    Close(Mark),
    /// An atom node: a mention or a citation (its marks come from context).
    Atom(Inline),
    /// `<br>`.
    Break,
    /// The closing tag of an atom, which carries nothing.
    Ignored,
    /// Any other tag, or one of ours with invalid attributes: kept as text.
    Other,
}

/// Reads one raw HTML tag as markdown-rs reports it.
pub(crate) fn parse_tag(raw: &str) -> Tag {
    let raw = raw.trim();
    if let Some(name) = raw.strip_prefix("</") {
        return close_tag(name.trim_end_matches('>').trim());
    }
    let Some(body) = raw
        .strip_prefix('<')
        .and_then(|rest| rest.strip_suffix('>'))
    else {
        return Tag::Other;
    };
    let body = body.trim_end_matches('/').trim_end();
    let name_end = body.find(char::is_whitespace).unwrap_or(body.len());
    let (name, rest) = body.split_at(name_end);
    let attributes = Attributes::parse(rest);
    match name {
        "u" => Tag::Open(Mark::Underline),
        "br" => Tag::Break,
        "span" => origin(&attributes).map_or(Tag::Other, |attrs| Tag::Open(Mark::Origin { attrs })),
        "mention" => mention(&attributes).map_or(Tag::Other, Tag::Atom),
        "citation" => citation(&attributes).map_or(Tag::Other, Tag::Atom),
        _ => Tag::Other,
    }
}

fn close_tag(name: &str) -> Tag {
    match name {
        "u" => Tag::Close(Mark::Underline),
        "span" => Tag::Close(Mark::Origin {
            attrs: OriginAttrs::USER,
        }),
        // `<mention …></mention>`: the opening tag already made the atom.
        "mention" | "citation" => Tag::Ignored,
        _ => Tag::Other,
    }
}

fn origin(attributes: &Attributes) -> Option<OriginAttrs> {
    let author = match attributes.get("data-origin")? {
        "user" => Author::User,
        "ai" => Author::Ai,
        _ => return None,
    };
    let edited = attributes.get("data-edited") == Some("true");
    Some(OriginAttrs { author, edited })
}

fn mention(attributes: &Attributes) -> Option<Inline> {
    let id: PersonId = attributes.get("data-id")?.parse().ok()?;
    let label = attributes.get("data-label")?.to_owned();
    (!label.is_empty()).then(|| Inline::Mention {
        attrs: MentionAttrs { id, label },
        marks: vec![],
    })
}

fn citation(attributes: &Attributes) -> Option<Inline> {
    let segment_id: SegmentId = attributes.get("data-segment-id")?.parse().ok()?;
    let start_ms = Millis(attributes.get("data-start-ms")?.parse().ok()?);
    Some(Inline::Citation {
        attrs: CitationAttrs {
            segment_id,
            start_ms,
        },
        marks: vec![],
    })
}

/// The opening tag of a mark that Markdown cannot express, if any.
pub(crate) fn open(mark: &Mark) -> Option<String> {
    match mark {
        Mark::Underline => Some("<u>".into()),
        Mark::Origin { attrs } => {
            let author = match attrs.author {
                Author::User => "user",
                Author::Ai => "ai",
            };
            let edited = if attrs.edited {
                r#" data-edited="true""#
            } else {
                ""
            };
            Some(format!(r#"<span data-origin="{author}"{edited}>"#))
        }
        _ => None,
    }
}

/// The closing tag matching [`open`].
pub(crate) fn close(mark: &Mark) -> Option<&'static str> {
    match mark {
        Mark::Underline => Some("</u>"),
        Mark::Origin { .. } => Some("</span>"),
        _ => None,
    }
}

/// `<mention … />` for a mention chip.
pub(crate) fn mention_tag(attrs: &MentionAttrs) -> String {
    format!(
        r#"<mention data-id="{}" data-label="{}" />"#,
        attrs.id,
        escape(&attrs.label)
    )
}

/// `<citation … />` for a transcript citation.
pub(crate) fn citation_tag(attrs: &CitationAttrs) -> String {
    format!(
        r#"<citation data-segment-id="{}" data-start-ms="{}" />"#,
        attrs.segment_id,
        attrs.start_ms.get()
    )
}

/// Escapes an attribute value for a double-quoted attribute.
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Reverses [`escape`] (and the apostrophe entity other writers use).
fn unescape(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// A tag's `name="value"` attributes, values unescaped.
struct Attributes(Vec<(String, String)>);

impl Attributes {
    fn parse(mut rest: &str) -> Self {
        let mut pairs = Vec::new();
        while let Some((name, after_name)) = rest.trim_start().split_once("=\"") {
            let Some((value, after_value)) = after_name.split_once('"') else {
                break;
            };
            pairs.push((name.trim().to_owned(), unescape(value)));
            rest = after_value;
        }
        Self(pairs)
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

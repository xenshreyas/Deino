//! Email drafts as `mailto:` URLs (RFC 6068), which the user's mail app
//! opens: the notes addressed to the other attendees ([`url`]), and any
//! composed message with To, CC and BCC ([`compose`], NOT-07's follow-ups).

use crate::{MeetingExport, RenderOptions, text};

/// The longest body a draft carries, in characters; longer notes are cut at
/// a line break, because mail apps reject very long URLs.
pub const MAX_BODY_CHARS: usize = 8_000;
/// Appended when the body was cut.
const SHORTENED: &str = "…\n\n(Shortened. Export the meeting from Deino for the full notes.)";

/// The `mailto:` URL of a draft with the meeting's title as its subject and
/// its notes (as plain text) as its body.
pub fn url(export: &MeetingExport, options: &RenderOptions) -> String {
    let recipients: Vec<String> = export
        .attendees
        .iter()
        .filter(|person| !person.is_me)
        .filter_map(|person| person.email.as_deref().map(encode))
        .collect();
    let body = shorten(&text::render(export, options)).replace('\n', "\r\n");
    format!(
        "mailto:{}?subject={}&body={}",
        recipients.join(","),
        encode(&export.title),
        encode(&body)
    )
}

/// A composed email: recipients, subject and plain-text body.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Compose<'a> {
    /// The To addresses.
    pub to: &'a [&'a str],
    /// The CC addresses.
    pub cc: &'a [&'a str],
    /// The BCC addresses.
    pub bcc: &'a [&'a str],
    /// The subject line.
    pub subject: &'a str,
    /// The body; line breaks become CRLF, as RFC 6068 asks.
    pub body: &'a str,
}

/// The `mailto:` URL of `draft`. Every address and field is percent-encoded
/// on its own, so a `?`, `&`, `,` or line break in any of them cannot add a
/// header or a recipient. Empty fields are left out.
pub fn compose(draft: &Compose<'_>) -> String {
    let body = draft.body.replace("\r\n", "\n").replace('\n', "\r\n");
    let fields = [
        ("cc", addresses(draft.cc)),
        ("bcc", addresses(draft.bcc)),
        ("subject", encode(draft.subject)),
        ("body", encode(&body)),
    ];
    let query: Vec<String> = fields
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    let to = addresses(draft.to);
    if query.is_empty() {
        return format!("mailto:{to}");
    }
    format!("mailto:{to}?{}", query.join("&"))
}

/// The addresses, each encoded on its own, joined by commas.
fn addresses(list: &[&str]) -> String {
    let encoded: Vec<String> = list.iter().map(|address| encode(address.trim())).collect();
    encoded.join(",")
}

/// `body`, cut to [`MAX_BODY_CHARS`] at the last line break before the limit.
fn shorten(body: &str) -> String {
    if body.chars().count() <= MAX_BODY_CHARS {
        return body.to_owned();
    }
    let head: String = body.chars().take(MAX_BODY_CHARS).collect();
    let cut = head
        .rfind('\n')
        .map_or(head.as_str(), |end| head.get(..end).unwrap_or(&head));
    format!("{cut}{SHORTENED}")
}

/// Percent-encodes everything but RFC 3986's unreserved characters.
fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push('%');
            out.push(hex_digit(byte >> 4));
            out.push(hex_digit(byte & 0x0F));
        }
    }
    out
}

/// An upper-case hex digit for a nibble (`0..16`).
fn hex_digit(nibble: u8) -> char {
    char::from_digit(u32::from(nibble), 16).map_or('0', |digit| digit.to_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExportSections;
    use crate::test_support::fixture_export;

    #[test]
    fn the_draft_is_addressed_to_the_other_attendees() {
        let options = RenderOptions {
            sections: ExportSections::SHARE,
            ..RenderOptions::default()
        };
        let url = url(&fixture_export("standup"), &options);

        assert!(url.starts_with("mailto:maya%40deino.dev,diego%40deino.dev,priya%40deino.dev,tom%40deino.dev?subject=Eng%20standup&body=Eng%20standup%0D%0A"));
        assert!(!url.contains("shreyas%40"));
        assert!(!url.contains(' ') && !url.contains('\n'));
    }

    #[test]
    fn long_notes_are_cut_at_a_line_break() {
        let body = format!("{}\n{}", "a".repeat(MAX_BODY_CHARS - 10), "b".repeat(50));
        let short = shorten(&body);
        assert!(short.starts_with(&"a".repeat(MAX_BODY_CHARS - 10)));
        assert!(short.ends_with(SHORTENED));
        assert!(!short.contains('b'));
    }

    #[test]
    fn a_composed_draft_encodes_each_field_on_its_own() {
        let url = compose(&Compose {
            to: &["marcus@brightkite.example", "hannah@brightkite.example"],
            cc: &["lena@deino.dev"],
            bcc: &[],
            subject: "Pilot & next steps?",
            body: "Hi Marcus,\n\nSee [deck link].\nbcc=eve@evil.example",
        });

        assert_eq!(
            url,
            "mailto:marcus%40brightkite.example,hannah%40brightkite.example\
             ?cc=lena%40deino.dev&subject=Pilot%20%26%20next%20steps%3F\
             &body=Hi%20Marcus%2C%0D%0A%0D%0ASee%20%5Bdeck%20link%5D.%0D%0Abcc%3Deve%40evil.example"
        );
    }

    #[test]
    fn an_address_cannot_smuggle_a_header_and_empty_fields_are_left_out() {
        let url = compose(&Compose {
            to: &["a@b.example?bcc=eve@evil.example"],
            ..Compose::default()
        });

        assert_eq!(url, "mailto:a%40b.example%3Fbcc%3Deve%40evil.example");
        assert_eq!(compose(&Compose::default()), "mailto:");
    }

    #[test]
    fn encoding_keeps_only_unreserved_characters() {
        assert_eq!(encode("Q4 & café?"), "Q4%20%26%20caf%C3%A9%3F");
    }
}

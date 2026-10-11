//! The whole meeting as JSON: every field of the [`MeetingExport`], with a
//! format name and version so scripts can rely on its shape.

use serde::Serialize;

use crate::{MeetingExport, RenderError};

/// The name of the document format.
pub const FORMAT: &str = "deino.meeting";
/// Its version; bump it when a field changes meaning or goes away.
pub const VERSION: u32 = 1;

/// The JSON document.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Document<'a> {
    format: &'static str,
    version: u32,
    link: String,
    meeting: &'a MeetingExport,
}

/// The meeting as pretty-printed JSON.
pub fn render(export: &MeetingExport) -> Result<String, RenderError> {
    let document = Document {
        format: FORMAT,
        version: VERSION,
        link: export.link(),
        meeting: export,
    };
    let mut json = serde_json::to_string_pretty(&document).map_err(RenderError::Json)?;
    json.push('\n');
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::fixture_export;

    #[test]
    fn the_standup_renders_as_the_full_object() {
        let json = render(&fixture_export("standup")).unwrap();
        insta::assert_snapshot!("json_standup", json);
    }

    #[test]
    fn the_document_names_its_format_and_link() {
        let json: serde_json::Value =
            serde_json::from_str(&render(&fixture_export("one-on-one")).unwrap()).unwrap();

        assert_eq!(json["format"], "deino.meeting");
        assert_eq!(json["version"], 1);
        assert_eq!(json["link"], "deino://meetings/meeting-fixture-one-on-one");
        assert_eq!(json["meeting"]["startedAt"], "2026-10-07T09:01:38-07:00");
    }
}

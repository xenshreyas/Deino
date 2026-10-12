//! Apple Vision text recognition (`VNRecognizeTextRequest`) on a snapshot
//! file, through cidre's `vn` bindings (the rev already in the tree).
//!
//! Accurate level, English, with language correction: slides and documents
//! read better than with the fast level, and a snapshot is read once, after
//! the meeting, off the runtime. Vision needs no permission and runs on
//! this Mac only; nothing is logged but counts.

use std::path::Path;

use cidre::{arc, ns, objc, vn};

use crate::error::ScreenError;

/// The language read (Deino is English only).
const LANGUAGE: &str = "en-US";

/// The text Vision reads in the image file at `path`, one observation per
/// line, top to bottom as Vision orders them; `""` when there is none.
pub(crate) fn recognize_text(path: &Path) -> Result<String, ScreenError> {
    let path = path.to_str().ok_or(ScreenError::Native {
        call: "a snapshot path that is not UTF-8",
    })?;
    // The pool drains what Vision autoreleases; its result must be `Clone`.
    objc::ar_pool(|| read(path)).map_err(|call| ScreenError::Native { call })
}

fn read(path: &str) -> Result<String, &'static str> {
    let url = ns::Url::with_fs_path_str(path, false);
    let handler = vn::ImageRequestHandler::with_url(&url, None);
    let request = text_request();
    let as_request: &vn::Request = &request;
    let requests = ns::Array::from_slice(&[as_request]);
    handler
        .perform(&requests)
        .map_err(|_| "VNImageRequestHandler performRequests")?;
    let Some(observations) = request.results() else {
        return Ok(String::new());
    };
    let lines: Vec<String> = observations
        .iter()
        .filter_map(|observation| {
            let candidates = observation.top_candidates(1);
            candidates.first().map(|text| text.string().to_string())
        })
        .collect();
    Ok(lines.join("\n"))
}

/// An accurate, English, language-corrected text request.
fn text_request() -> arc::R<vn::RecognizeTextRequest> {
    let mut request = vn::RecognizeTextRequest::new();
    request.set_recognition_level(vn::RequestTextRecognitionLevel::Accurate);
    request.set_uses_lang_correction(true);
    let language = ns::String::with_str(LANGUAGE);
    request.set_recognition_langs(&ns::Array::from_slice(&[language.as_ref()]));
    request
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn slide(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/screen")
            .join(name)
    }

    /// Real Vision on a fixture slide: runs headless (no window server or
    /// permission needed), so it runs wherever the macOS tests run.
    #[test]
    fn vision_reads_a_slide() {
        let text = recognize_text(&slide("slide-1.png")).unwrap();

        assert!(text.contains("Q3 Business Review"), "{text}");
        assert!(text.contains("Revenue grew 40%"), "{text}");
    }

    #[test]
    fn a_missing_picture_is_an_error() {
        assert!(recognize_text(&slide("no-such-slide.png")).is_err());
    }
}

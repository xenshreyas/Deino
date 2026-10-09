//! The notes feature's types: a meeting's notes, and the images in them.

use std::fmt;

use chrono::{DateTime, Utc};
use deino_doc::Doc;
use deino_domain::MeetingId;
use serde::Serialize;
use uuid::Uuid;

/// The largest note document accepted, as stored JSON: about a novel's worth
/// of text. Images are references, so they do not count.
pub const MAX_DOC_BYTES: usize = 2 * 1024 * 1024;

/// The largest image a note accepts.
pub const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

/// The folder inside a meeting's folder that holds its note images.
pub const IMAGES_FOLDER: &str = "images";

/// A meeting's notes as the notepad opens them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MeetingNotes {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The document; one empty paragraph when nothing was written yet.
    pub doc: Doc,
    /// When the notes were last saved; `None` before the first save.
    pub updated_at: Option<DateTime<Utc>>,
    /// The meeting's folder, which image sources (`images/<name>`) are
    /// relative to; the UI turns them into asset URLs.
    pub folder: String,
}

/// What a save reports back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SavedNotes {
    /// The meeting whose notes were saved.
    pub meeting_id: MeetingId,
    /// When they were saved.
    pub updated_at: DateTime<Utc>,
}

/// An image stored for a meeting's notes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
pub struct NoteImage {
    /// The source to put in the image node: `images/<name>`.
    pub src: String,
}

/// A meeting's notes as stored: the document and the Markdown derived from it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredNotes {
    /// The meeting.
    pub meeting_id: MeetingId,
    /// The document, normalized and valid.
    pub doc: Doc,
    /// [`deino_doc::Flavor::Readable`] Markdown of `doc`, for LLM context,
    /// search and export.
    pub markdown: String,
    /// When the notes were first saved (kept by later saves).
    pub created_at: DateTime<Utc>,
    /// When they were last saved.
    pub updated_at: DateTime<Utc>,
}

/// The image formats a note accepts, recognised by content, not by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageKind {
    /// PNG (what macOS screenshots and the clipboard produce).
    Png,
    /// JPEG.
    Jpeg,
    /// GIF.
    Gif,
    /// WebP.
    Webp,
}

impl ImageKind {
    /// Every accepted format.
    pub const ALL: [Self; 4] = [Self::Png, Self::Jpeg, Self::Gif, Self::Webp];

    /// The format of `bytes`, from their magic number.
    pub fn sniff(bytes: &[u8]) -> Option<Self> {
        let starts = |magic: &[u8]| bytes.starts_with(magic);
        if starts(b"\x89PNG\r\n\x1a\n") {
            Some(Self::Png)
        } else if starts(b"\xff\xd8\xff") {
            Some(Self::Jpeg)
        } else if starts(b"GIF87a") || starts(b"GIF89a") {
            Some(Self::Gif)
        } else if starts(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
            Some(Self::Webp)
        } else {
            None
        }
    }

    /// The file extension, without the dot.
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Gif => "gif",
            Self::Webp => "webp",
        }
    }
}

/// A stored image's file name: `<UUIDv7>.<extension>`. Nothing else can be
/// named, so an image source can never point outside the images folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageName {
    id: Uuid,
    kind: ImageKind,
}

impl ImageName {
    /// A name for a new image of `kind`.
    pub fn new(id: Uuid, kind: ImageKind) -> Self {
        Self { id, kind }
    }

    /// Reads an image node's `src` (`images/<uuid>.<ext>`), if it names a
    /// stored image.
    pub fn from_src(src: &str) -> Option<Self> {
        let file = src.strip_prefix(IMAGES_FOLDER)?.strip_prefix('/')?;
        let (stem, extension) = file.split_once('.')?;
        let kind = ImageKind::ALL
            .into_iter()
            .find(|kind| kind.extension() == extension)?;
        let id = Uuid::try_parse(stem).ok()?;
        (id.hyphenated().to_string() == stem).then_some(Self { id, kind })
    }

    /// The `src` an image node uses for this image.
    pub fn src(&self) -> String {
        format!("{IMAGES_FOLDER}/{self}")
    }
}

impl fmt::Display for ImageName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}.{}",
            self.id.hyphenated(),
            self.kind.extension()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "0192f3a0-6c1e-7b2a-9d4e-1f2a3b4c5d6e";

    #[test]
    fn image_kinds_are_recognised_by_their_bytes() {
        assert_eq!(
            ImageKind::sniff(b"\x89PNG\r\n\x1a\nrest"),
            Some(ImageKind::Png)
        );
        assert_eq!(ImageKind::sniff(b"\xff\xd8\xff\xe0"), Some(ImageKind::Jpeg));
        assert_eq!(ImageKind::sniff(b"GIF89a.."), Some(ImageKind::Gif));
        assert_eq!(
            ImageKind::sniff(b"RIFF\0\0\0\0WEBPVP8 "),
            Some(ImageKind::Webp)
        );
        assert_eq!(ImageKind::sniff(b"<svg xmlns="), None);
        assert_eq!(ImageKind::sniff(b""), None);
    }

    #[test]
    fn image_sources_round_trip_through_names() {
        let name = ImageName::new(Uuid::parse_str(ID).unwrap(), ImageKind::Png);

        assert_eq!(name.src(), format!("images/{ID}.png"));
        assert_eq!(ImageName::from_src(&name.src()), Some(name));
    }

    #[test]
    fn only_stored_image_names_are_sources() {
        for src in [
            "../../etc/passwd",
            "images/../secret.png",
            "/Users/me/x.png",
            "https://example.com/a.png",
            "images/not-a-uuid.png",
            &format!("images/{ID}.svg"),
            &format!("images/{}.png", ID.to_uppercase()),
            &format!("images/{ID}.png/x"),
        ] {
            assert_eq!(ImageName::from_src(src), None, "{src}");
        }
    }
}

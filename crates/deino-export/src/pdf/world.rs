//! The typesetter's world: one template, one data file, the bundled Inter.
//!
//! Nothing comes from the system: no fonts (so every Mac renders the same
//! PDF), no files beyond `data.json`, no packages, no clock. The fonts and
//! the standard library are parsed once per process.

use std::sync::LazyLock;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

/// The document template (Typst markup).
const TEMPLATE: &str = include_str!("template.typ");
/// The file the template reads the meeting from.
const DATA_PATH: &str = "data.json";

/// Inter 4 in the four styles the template uses: static instances of the
/// variable font's latin and latin-ext subsets (SIL OFL 1.1, `fonts/OFL.txt`).
static FONT_FILES: [&[u8]; 4] = [
    include_bytes!("../../fonts/Inter-Regular.ttf"),
    include_bytes!("../../fonts/Inter-SemiBold.ttf"),
    include_bytes!("../../fonts/Inter-Italic.ttf"),
    include_bytes!("../../fonts/Inter-SemiBoldItalic.ttf"),
];

/// The parsed fonts and the book the layout engine looks them up in.
static FONTS: LazyLock<(Vec<Font>, LazyHash<FontBook>)> = LazyLock::new(|| {
    let fonts: Vec<Font> = FONT_FILES
        .iter()
        .flat_map(|file| Font::iter(Bytes::new(*file)))
        .collect();
    let book = LazyHash::new(FontBook::from_fonts(&fonts));
    (fonts, book)
});

/// Typst's standard library, built once.
static LIBRARY: LazyLock<LazyHash<Library>> =
    LazyLock::new(|| LazyHash::new(<Library as LibraryExt>::default()));

/// One PDF's world: the template as `main.typ` and the meeting as `data.json`.
pub(crate) struct PrintWorld {
    main: Source,
    data_id: FileId,
    data: Bytes,
}

impl PrintWorld {
    /// A world whose `data.json` holds `data`; `None` only if the constant
    /// data path were invalid.
    pub(crate) fn new(data: Vec<u8>) -> Option<Self> {
        let path = VirtualPath::new(DATA_PATH).ok()?;
        Some(Self {
            main: Source::detached(TEMPLATE),
            data_id: RootedPath::new(VirtualRoot::Project, path).intern(),
            data: Bytes::new(data),
        })
    }

    fn missing(id: FileId) -> FileError {
        FileError::NotFound(id.vpath().get_without_slash().into())
    }
}

impl World for PrintWorld {
    fn library(&self) -> &LazyHash<Library> {
        &LIBRARY
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &FONTS.1
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            Ok(self.main.clone())
        } else {
            Err(Self::missing(id))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.data_id {
            Ok(self.data.clone())
        } else {
            Err(Self::missing(id))
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        FONTS.0.get(index).cloned()
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_fonts_are_inter_in_four_styles() {
        let book = &FONTS.1;
        let families: Vec<String> = (0..FONTS.0.len())
            .filter_map(|index| book.info(index).map(|info| info.family.clone()))
            .collect();
        assert_eq!(families, ["Inter"; 4]);
    }
}

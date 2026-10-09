//! Re-deflates a DOCX package: `docx-rs` stores every part uncompressed,
//! which makes a long transcript's document ten times larger than Word's.

use std::io::{Cursor, Read, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// `package` with every file deflated, in the same order (Word wants
/// `[Content_Types].xml` first, which `docx-rs` already writes first).
pub(crate) fn deflate(package: &[u8]) -> zip::result::ZipResult<Vec<u8>> {
    let mut archive = ZipArchive::new(Cursor::new(package))?;
    let mut writer = ZipWriter::new(Cursor::new(Vec::with_capacity(package.len() / 4)));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut buffer = Vec::new();
    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        if file.is_dir() {
            continue;
        }
        buffer.clear();
        file.read_to_end(&mut buffer)?;
        writer.start_file(file.name(), options)?;
        writer.write_all(&buffer)?;
    }
    Ok(writer.finish()?.into_inner())
}

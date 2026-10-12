//! Snapshots as PNG files: [`encode_png`] turns a [`Frame`] into PNG bytes
//! (RGB, the window's alpha dropped: a window capture is opaque), and
//! [`write_atomically`] puts them on disk under a temporary name first, so
//! a crash never leaves half a picture under the final name.

use std::fs;
use std::io;
use std::path::Path;

use crate::frame::Frame;

/// Encodes `frame` as an 8-bit RGB PNG.
pub fn encode_png(frame: &Frame) -> Result<Vec<u8>, png::EncodingError> {
    let mut rgb = Vec::with_capacity(frame.bgra().len() / 4 * 3);
    for pixel in frame.bgra().chunks_exact(4) {
        if let &[b, g, r, _] = pixel {
            rgb.extend_from_slice(&[r, g, b]);
        }
    }
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, frame.width(), frame.height());
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&rgb)?;
    writer.finish()?;
    Ok(bytes)
}

/// Writes `bytes` to `path` through `<path>.tmp` and a rename, creating the
/// folder when needed.
pub fn write_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    fs::write(&temporary, bytes)?;
    fs::rename(&temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_round_trips_through_png() {
        let frame = Frame::solid(4, 3, [10, 20, 30, 255]).unwrap();
        let bytes = encode_png(&frame).unwrap();

        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder.read_info().unwrap();
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut pixels).unwrap();

        assert_eq!((info.width, info.height), (4, 3));
        assert_eq!(info.color_type, png::ColorType::Rgb);
        assert_eq!(&pixels[..3], &[30, 20, 10], "BGRA becomes RGB");
    }

    #[test]
    fn files_are_written_whole_into_a_new_folder() {
        let root = std::env::temp_dir().join(format!("deino-screen-{}", std::process::id()));
        let path = root.join("snapshots").join("1000.png");

        write_atomically(&path, b"png").unwrap();

        assert_eq!(fs::read(&path).unwrap(), b"png");
        assert!(!root.join("snapshots").join("1000.png.tmp").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
